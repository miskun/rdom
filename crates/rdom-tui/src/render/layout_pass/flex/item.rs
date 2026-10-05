//! Flex items — CSS Flexible Box §4: what a flex container lays out.
//!
//! "Each in-flow child of a flex container becomes a flex item, and
//! each contiguous sequence of child text runs is wrapped in an
//! anonymous block container flex item. However, if the entire sequence
//! of child text runs contains only white space […] it is instead not
//! rendered." The children are the box tree's
//! (`box_tree::flex_sequence`): a box-less child's children and
//! pseudo-elements are the container's, and the container's own
//! `::before` / `::after` are child boxes — blockified, so items too.
//!
//! An element item is its element. An anonymous item
//! ([`AnonymousItem`]) is a run of the sequence — text nodes, or one
//! pseudo-element — with the style of an anonymous box (inherited
//! properties from the container, every other one initial: `order: 0`,
//! `flex: 0 1 auto`, `auto` sizes, no margins, padding or border). Its
//! content is packed as an inline formatting context of its own and
//! stored on the container as an `AnonymousIfc`, which paint, hit-
//! testing, the caret and selection read as they read a block
//! container's anonymous block boxes.

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::render::inline::{InlineLayout, RunPseudos, pack_run};
use crate::style::ComputedStyle;

/// One flex item of a container.
#[derive(Debug, Clone)]
pub(in crate::render::layout_pass) enum FlexItem {
    /// An in-flow element child (through box-less children).
    Element(NodeId),
    /// An anonymous block container item.
    Anonymous(Rc<AnonymousItem>),
}

/// An anonymous flex item: a run of its container's box sequence.
#[derive(Debug)]
pub(in crate::render::layout_pass) struct AnonymousItem {
    /// The flex container.
    container: NodeId,
    /// The run: text nodes, or one generated item.
    content: Vec<BoxItem>,
    /// `[start, end)` in the container's `flex_sequence`.
    child_range: (usize, usize),
    /// The anonymous box's computed style.
    style: Rc<ComputedStyle>,
}

impl FlexItem {
    /// The element, for an element item.
    pub(in crate::render::layout_pass) fn node(&self) -> Option<NodeId> {
        match self {
            FlexItem::Element(id) => Some(*id),
            FlexItem::Anonymous(_) => None,
        }
    }

    /// The item's computed style: its element's, or the anonymous box's.
    pub(in crate::render::layout_pass) fn computed(&self, dom: &Dom<TuiExt>) -> Rc<ComputedStyle> {
        match self {
            FlexItem::Element(id) => dom
                .node(*id)
                .computed_rc()
                .unwrap_or_else(|| Rc::new(ComputedStyle::initial())),
            FlexItem::Anonymous(a) => a.style.clone(),
        }
    }

    /// The box the item is laid out in: its flex container.
    pub(in crate::render::layout_pass) fn box_parent(&self, dom: &Dom<TuiExt>) -> Option<NodeId> {
        match self {
            FlexItem::Element(id) => crate::render::box_tree::box_parent(dom, *id),
            FlexItem::Anonymous(a) => Some(a.container),
        }
    }

    /// Whether the height its percentages resolve against is definite
    /// (`block::nearest_block_ancestor_height_is_definite`).
    pub(in crate::render::layout_pass) fn height_basis_is_definite(
        &self,
        dom: &Dom<TuiExt>,
    ) -> bool {
        use crate::render::layout_pass::block::{
            height_is_definite_below, nearest_block_ancestor_height_is_definite,
        };
        match self {
            FlexItem::Element(id) => nearest_block_ancestor_height_is_definite(dom, *id),
            FlexItem::Anonymous(a) => height_is_definite_below(dom, Some(a.container)),
        }
    }

    /// The item's `order` (CSS Flexbox §5.4): an anonymous item's is 0.
    pub(in crate::render::layout_pass) fn order(&self, dom: &Dom<TuiExt>) -> i32 {
        match self {
            FlexItem::Element(id) => crate::render::box_tree::order_of(dom, *id),
            FlexItem::Anonymous(_) => 0,
        }
    }

    /// `visibility: collapse` on the item (Flexbox §4.4) — an anonymous
    /// item inherits its container's `visibility`.
    pub(in crate::render::layout_pass) fn is_collapsed(&self, dom: &Dom<TuiExt>) -> bool {
        match self {
            FlexItem::Element(id) => super::is_collapsed(dom, *id),
            FlexItem::Anonymous(a) => a.style.visibility == crate::layout::Visibility::Collapse,
        }
    }
}

impl AnonymousItem {
    /// The item's content packed `width` cells wide.
    pub(in crate::render::layout_pass) fn pack(
        &self,
        dom: &Dom<TuiExt>,
        width: u16,
    ) -> InlineLayout {
        pack_run(
            dom,
            self.container,
            &self.content,
            RunPseudos::default(),
            width,
        )
    }

    /// The widest line of the content packed `width` cells wide.
    fn widest_line(&self, dom: &Dom<TuiExt>, width: u16) -> u16 {
        self.pack(dom, width)
            .lines
            .iter()
            .map(|l| l.width)
            .max()
            .unwrap_or(0)
    }

    /// The item's content size along `direction` (it has no padding,
    /// border or declared size): on the inline axis its max-content
    /// width (unwrapped) or min-content width (broken at every soft wrap
    /// opportunity, CSS Sizing 3 §4.1 / §4.2); on the block axis the rows
    /// it packs to at `width`.
    pub(in crate::render::layout_pass) fn content_size(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        width: u16,
        max_content: bool,
    ) -> u16 {
        match direction {
            Direction::Row if max_content => self.widest_line(dom, u16::MAX),
            Direction::Row => self.widest_line(dom, 0),
            Direction::Column => self.pack(dom, width).height(),
        }
    }

    /// Its first and last content rows (baselines, CSS Box Alignment 3
    /// §9.1) packed `width` cells wide: its first and last lines.
    pub(in crate::render::layout_pass) fn content_rows(
        &self,
        dom: &Dom<TuiExt>,
        width: u16,
    ) -> Option<(u16, u16)> {
        let rows = self.pack(dom, width).height();
        (rows > 0).then(|| (0, rows - 1))
    }

    /// Lay the item out at `rect`, its border box (which is its content
    /// box): its content packed at its width.
    pub(in crate::render::layout_pass) fn lay_out(
        &self,
        dom: &Dom<TuiExt>,
        rect: LayoutRect,
    ) -> AnonymousIfc {
        AnonymousIfc {
            rect,
            inline_layout: self.pack(dom, rect.width),
            child_range: self.child_range,
        }
    }
}

/// White space that the `white-space` property can affect (CSS Text 3
/// §4): a run of only these is not rendered as a flex item (Flexbox §4),
/// whatever `white-space` says.
fn is_document_white_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// The flex container `id`'s flex items, in document order (CSS Flexbox
/// §4): its in-flow elements (through box-less children and fragments),
/// each pseudo-element that generates a box, and an anonymous item per
/// contiguous run of text that is not all white space. Out-of-flow and
/// `display: none` children are no items and do not split a run.
pub(in crate::render::layout_pass) fn flex_items(dom: &Dom<TuiExt>, id: NodeId) -> Vec<FlexItem> {
    let sequence = crate::render::box_tree::flex_sequence(dom, id);
    let mut b = ItemsBuilder {
        dom,
        container: id,
        style: None,
        items: Vec::new(),
        run: Vec::new(),
        run_start: 0,
        run_end: 0,
        run_renders: false,
    };
    for (i, &entry) in sequence.iter().enumerate() {
        match entry {
            BoxItem::Generated(..) => {
                b.close_run();
                b.push_anonymous(vec![entry], (i, i + 1));
            }
            BoxItem::Node(n) => match dom.node(n).node_type() {
                NodeType::Text => b.extend_run(entry, i, n),
                NodeType::Element if crate::render::layout_pass::is_in_flow(dom, n) => {
                    b.close_run();
                    b.items.push(FlexItem::Element(n));
                }
                // Out-of-flow elements, comments: no item, no break.
                _ => {}
            },
        }
    }
    b.close_run();
    b.items
}

/// [`flex_items`]' state: the items so far, and the open text run.
struct ItemsBuilder<'a> {
    dom: &'a Dom<TuiExt>,
    container: NodeId,
    /// The anonymous box style, computed for the first anonymous item.
    style: Option<Rc<ComputedStyle>>,
    items: Vec<FlexItem>,
    run: Vec<BoxItem>,
    run_start: usize,
    run_end: usize,
    /// Some text of the run is other than white space.
    run_renders: bool,
}

impl ItemsBuilder<'_> {
    fn push_anonymous(&mut self, content: Vec<BoxItem>, child_range: (usize, usize)) {
        let dom = self.dom;
        let container = self.container;
        let style = self
            .style
            .get_or_insert_with(|| {
                let parent = dom
                    .node(container)
                    .computed_rc()
                    .unwrap_or_else(|| Rc::new(ComputedStyle::initial()));
                Rc::new(crate::style::cascade::anonymous_box_style(&parent))
            })
            .clone();
        self.items.push(FlexItem::Anonymous(Rc::new(AnonymousItem {
            container,
            content,
            child_range,
            style,
        })));
    }

    fn extend_run(&mut self, entry: BoxItem, i: usize, text: NodeId) {
        if self.run.is_empty() {
            self.run_start = i;
        }
        self.run.push(entry);
        self.run_end = i + 1;
        self.run_renders |= self
            .dom
            .node(text)
            .node_value()
            .is_some_and(|t| !t.chars().all(is_document_white_space));
    }

    fn close_run(&mut self) {
        let run = std::mem::take(&mut self.run);
        if self.run_renders {
            self.push_anonymous(run, (self.run_start, self.run_end));
        }
        self.run_renders = false;
    }
}

/// Sort `items` — flex items in document order — into order-modified
/// document order (CSS Flexbox §5.4): ascending `order`, document order
/// among equals (a stable sort). No-op when every `order` is 0.
pub(in crate::render::layout_pass) fn sort_by_order(dom: &Dom<TuiExt>, items: &mut [FlexItem]) {
    if items.iter().any(|c| c.order(dom) != 0) {
        items.sort_by_key(|c| c.order(dom));
    }
}

/// `items` as element items (a block container's children, measured as
/// flex items are).
pub(in crate::render::layout_pass) fn elements(ids: &[NodeId]) -> Vec<FlexItem> {
    ids.iter().map(|&id| FlexItem::Element(id)).collect()
}
