//! The items a flex or grid container lays out — CSS Flexible Box §4,
//! CSS Grid 2 §6.1.
//!
//! "Each in-flow child of a flex container becomes a flex item, and
//! each contiguous sequence of child text runs is wrapped in an
//! anonymous block container flex item. However, if the entire sequence
//! of child text runs contains only white space […] it is instead not
//! rendered." Grid §6.1 says the same of grid items. The children are
//! the box tree's
//! (`box_tree::item_sequence`): a box-less child's children and
//! pseudo-elements are the container's, and the container's own
//! `::before` / `::after` are child boxes — blockified, so items too.
//!
//! An element item is its element. An [`AnonymousItem`] is a run of the
//! sequence with no node of its own: a run of text nodes, with the style
//! of an anonymous box (inherited properties from the container, every
//! other one initial: `order: 0`, `flex: 0 1 auto`, `auto` placement, `auto` sizes, no
//! margins, padding or border), or one pseudo-element, a box with its
//! own computed style — its sizes, `flex`, `order`, margins, padding,
//! border and alignment apply as an element item's do. Its content is
//! packed as an inline formatting context of its own, inside its padding
//! and border, and stored on the container as an `AnonymousIfc` (a
//! pseudo-element's with its border box, `GeneratedBox`), which paint,
//! hit-testing, the caret and selection read as they read a block
//! container's anonymous block boxes.

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::Direction;
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::intrinsic::Keywords;
use crate::style::ComputedStyle;

mod anonymous;
mod baseline;
mod minimum;

pub(in crate::render::layout_pass) use anonymous::AnonymousItem;
pub(in crate::render::layout_pass) use baseline::BaselineBox;
pub(in crate::render::layout_pass) use minimum::{Suggestion, content_based_minimum};

/// One flex or grid item of a container.
#[derive(Debug, Clone)]
pub(in crate::render::layout_pass) enum Item {
    /// An in-flow element child (through box-less children).
    Element(NodeId),
    /// An anonymous block container item.
    Anonymous(Rc<AnonymousItem>),
}

impl Item {
    /// The box `item` of the box tree as an item to measure and lay out:
    /// an element, or a `::before` / `::after` as its own box
    /// ([`AnonymousItem::pseudo`]). `None` for a pseudo-element with no
    /// computed style.
    pub(in crate::render::layout_pass) fn of_box(dom: &Dom<TuiExt>, item: BoxItem) -> Option<Self> {
        match item {
            BoxItem::Node(id) => Some(Item::Element(id)),
            BoxItem::Generated(host, slot) => {
                AnonymousItem::pseudo(dom, host, slot).map(|a| Item::Anonymous(Rc::new(a)))
            }
        }
    }

    /// The element, for an element item.
    pub(in crate::render::layout_pass) fn node(&self) -> Option<NodeId> {
        match self {
            Item::Element(id) => Some(*id),
            Item::Anonymous(_) => None,
        }
    }

    /// The item's computed style: its element's, or the anonymous box's.
    pub(in crate::render::layout_pass) fn computed(&self, dom: &Dom<TuiExt>) -> Rc<ComputedStyle> {
        match self {
            Item::Element(id) => dom
                .node(*id)
                .computed_rc()
                .unwrap_or_else(|| Rc::new(ComputedStyle::initial())),
            Item::Anonymous(a) => a.style_rc(),
        }
    }

    /// The box the item is laid out in: its flex or grid container.
    pub(in crate::render::layout_pass) fn box_parent(&self, dom: &Dom<TuiExt>) -> Option<NodeId> {
        match self {
            Item::Element(id) => crate::render::box_tree::box_parent(dom, *id),
            Item::Anonymous(a) => a.box_parent(),
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
            Item::Element(id) => nearest_block_ancestor_height_is_definite(dom, *id),
            Item::Anonymous(a) => height_is_definite_below(dom, Some(a.container())),
        }
    }

    /// The item's `order` (CSS Flexbox §5.4): an anonymous item's is 0,
    /// a pseudo-element's its own.
    pub(in crate::render::layout_pass) fn order(&self, dom: &Dom<TuiExt>) -> i32 {
        match self {
            Item::Element(id) => crate::render::box_tree::order_of(dom, *id),
            Item::Anonymous(a) => a.style().order,
        }
    }

    /// The item's [`Keywords`] on `direction`: its declared sizes through
    /// its `box-sizing`, its intrinsic keywords measured from its content.
    pub(in crate::render::layout_pass) fn keywords<'a>(
        &'a self,
        dom: &'a Dom<TuiExt>,
        computed: &ComputedStyle,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
    ) -> Keywords<'a> {
        match self {
            Item::Element(id) => {
                Keywords::new(dom, *id, computed, direction, cross_budget, cb_width)
            }
            Item::Anonymous(a) => Keywords::for_run(dom, a, direction, cross_budget, cb_width),
        }
    }

    /// The item's border-box content size along `direction`
    /// (`intrinsic::intrinsic_size` for an element), its block size
    /// measured at `cross_budget` wide.
    pub(in crate::render::layout_pass) fn intrinsic_size(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
    ) -> u16 {
        match self {
            Item::Element(id) => crate::render::layout_pass::intrinsic::intrinsic_size(
                dom,
                *id,
                direction,
                cross_budget,
                cb_width,
            ),
            Item::Anonymous(a) => a.content_size(dom, direction, cross_budget, true, cb_width),
        }
    }

    /// The item's max-content (`max_content`) or min-content border-box
    /// size along `direction` (CSS Sizing 3 §5.1).
    pub(in crate::render::layout_pass) fn content_extreme(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
        max_content: bool,
    ) -> u16 {
        use crate::render::layout_pass::intrinsic::{content_max_size, content_min_size};
        match self {
            Item::Element(id) if max_content => {
                content_max_size(dom, *id, direction, cross_budget, cb_width)
            }
            Item::Element(id) => content_min_size(dom, *id, direction, cross_budget, cb_width),
            Item::Anonymous(a) => {
                a.content_size(dom, direction, cross_budget, max_content, cb_width)
            }
        }
    }

    /// The item's intrinsic size contribution along `direction`, a
    /// border box (CSS Sizing 3 §5.2): its min-content contribution, or
    /// with `max_content` its max-content one — its declared size when
    /// definite, else its content size, clamped by its `min-*` / `max-*`.
    pub(in crate::render::layout_pass) fn contribution(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
        max_content: bool,
    ) -> u16 {
        match self {
            Item::Element(id) => crate::render::layout_pass::intrinsic::contribution(
                dom,
                *id,
                direction,
                cross_budget,
                cb_width,
                max_content,
            ),
            Item::Anonymous(a) => a.box_size(dom, direction, cross_budget, cb_width, max_content),
        }
    }

    /// `visibility: collapse` on the item (Flexbox §4.4) — an anonymous
    /// item inherits its container's `visibility`.
    pub(in crate::render::layout_pass) fn is_collapsed(&self, dom: &Dom<TuiExt>) -> bool {
        match self {
            Item::Element(id) => super::flex::is_collapsed(dom, *id),
            Item::Anonymous(a) => a.style().visibility == crate::layout::Visibility::Collapse,
        }
    }
}

/// White space that the `white-space` property can affect (CSS Text 3
/// §4): a run of only these is not rendered as a flex item (Flexbox §4),
/// whatever `white-space` says.
fn is_document_white_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// The flex or grid container `id`'s items, in document order (CSS Flexbox
/// §4): its in-flow elements (through box-less children and fragments),
/// each pseudo-element that generates a box, and an anonymous item per
/// contiguous run of text that is not all white space. Out-of-flow and
/// `display: none` children are no items and do not split a run.
pub(in crate::render::layout_pass) fn items_of(dom: &Dom<TuiExt>, id: NodeId) -> Vec<Item> {
    let sequence = crate::render::box_tree::item_sequence(dom, id);
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
            BoxItem::Generated(host, slot) => {
                b.close_run();
                b.push_generated(entry, (host, slot), (i, i + 1));
            }
            BoxItem::Node(n) => match dom.node(n).node_type() {
                NodeType::Text => b.extend_run(entry, i, n),
                NodeType::Element if crate::render::layout_pass::is_in_flow(dom, n) => {
                    b.close_run();
                    b.items.push(Item::Element(n));
                }
                // Out-of-flow elements, comments: no item, no break.
                _ => {}
            },
        }
    }
    b.close_run();
    b.items
}

/// [`items_of`]' state: the items so far, and the open text run.
struct ItemsBuilder<'a> {
    dom: &'a Dom<TuiExt>,
    container: NodeId,
    /// The anonymous box style, computed for the first anonymous item.
    style: Option<Rc<ComputedStyle>>,
    items: Vec<Item>,
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
        self.items.push(Item::Anonymous(Rc::new(AnonymousItem::new(
            container,
            content,
            child_range,
            style,
            None,
        ))));
    }

    /// A `::before` / `::after` item (CSS Flexbox §4: a child box,
    /// blockified): a box with the pseudo-element's computed style.
    fn push_generated(
        &mut self,
        entry: BoxItem,
        (host, slot): (NodeId, PseudoSlot),
        child_range: (usize, usize),
    ) {
        let ext = self.dom.node(host).ext();
        let style = ext.and_then(|e| match slot {
            PseudoSlot::Before => e.computed_before.clone(),
            PseudoSlot::After => e.computed_after.clone(),
        });
        let Some(style) = style else {
            // `box_tree::item_sequence` lists a pseudo-element only when it
            // has a computed style.
            debug_assert!(false, "a generated item has a computed style");
            self.push_anonymous(vec![entry], child_range);
            return;
        };
        self.items.push(Item::Anonymous(Rc::new(AnonymousItem::new(
            self.container,
            vec![entry],
            child_range,
            style,
            Some((host, slot)),
        ))));
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

/// Sort `items` — flex or grid items in document order — into order-modified
/// document order (CSS Flexbox §5.4): ascending `order`, document order
/// among equals (a stable sort). No-op when every `order` is 0.
pub(in crate::render::layout_pass) fn sort_by_order(dom: &Dom<TuiExt>, items: &mut [Item]) {
    if items.iter().any(|c| c.order(dom) != 0) {
        items.sort_by_key(|c| c.order(dom));
    }
}

/// `items` as element items (a block container's children, measured as
/// flex items are).
pub(in crate::render::layout_pass) fn elements(ids: &[NodeId]) -> Vec<Item> {
    ids.iter().map(|&id| Item::Element(id)).collect()
}
