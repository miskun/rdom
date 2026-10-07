//! A box with no node of its own (CSS Flexbox §4, CSS Grid 2 §6.1, CSS
//! Pseudo 4 §2): a text run in an anonymous box, or one `::before` /
//! `::after` in a box styled by its computed style — a flex or grid item,
//! an atomic inline or a float. It is measured and laid out by packing
//! its content inside its padding and border; a `::before` / `::after`
//! whose `display` makes it a flex or grid container lays its content —
//! one anonymous item wrapping its text ([`AnonymousItem::content_item`])
//! — out as one, through the flex and grid layout elements use.

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, GeneratedBox, PseudoSlot, TuiExt};
use crate::layout::{Direction, LayoutRect, Size};
use crate::render::box_tree::BoxItem;
use crate::render::inline::{InlineLayout, RunPseudos, pack_run};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::Keywords;
use crate::style::ComputedStyle;

/// An anonymous flex or grid item: a run of its container's box sequence.
#[derive(Debug)]
pub(in crate::render::layout_pass) struct AnonymousItem {
    /// The flex container.
    container: NodeId,
    /// The run: text nodes, or one generated item.
    content: Vec<BoxItem>,
    /// `[start, end)` in the container's `item_sequence`.
    child_range: (usize, usize),
    /// The box's computed style: the anonymous box style for a text
    /// run, the pseudo-element's own for a generated item.
    style: Rc<ComputedStyle>,
    /// The pseudo-element a generated item is (its host and slot).
    generated: Option<(NodeId, PseudoSlot)>,
    /// The one item of a generated flex or grid container, wrapping its
    /// text: laid out in that box, which scrolls nothing, not in
    /// `container`.
    in_generated: bool,
    /// [`Self::content_item`], built on first use.
    inner: std::cell::OnceCell<Option<super::Item>>,
}

/// A box's padding plus border on each side, in cells.
#[derive(Debug, Clone, Copy, Default)]
struct Edges {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl AnonymousItem {
    /// A box over `content` (`[start, end)` of the container's
    /// `item_sequence`), styled `style`; `generated` names the
    /// pseudo-element it is.
    pub(super) fn new(
        container: NodeId,
        content: Vec<BoxItem>,
        child_range: (usize, usize),
        style: Rc<ComputedStyle>,
        generated: Option<(NodeId, PseudoSlot)>,
    ) -> Self {
        Self {
            container,
            content,
            child_range,
            style,
            generated,
            in_generated: false,
            inner: std::cell::OnceCell::new(),
        }
    }

    /// `host`'s `slot` pseudo-element as a box of its own (CSS Pseudo 4
    /// §2), styled by its computed style: an atomic inline or a float of
    /// its host's flow. `None` when it has no computed style.
    pub(in crate::render::layout_pass) fn pseudo(
        dom: &Dom<TuiExt>,
        host: NodeId,
        slot: PseudoSlot,
    ) -> Option<Self> {
        let style = dom.node(host).ext()?.computed_pseudo(slot)?.clone();
        Some(Self::new(
            host,
            vec![BoxItem::Generated(host, slot)],
            (0, 0),
            style,
            Some((host, slot)),
        ))
    }

    /// The box it is laid out in: its flex or grid container, or — the
    /// item of a generated container — none with a node.
    pub(super) fn box_parent(&self) -> Option<NodeId> {
        (!self.in_generated).then_some(self.container)
    }

    /// The element whose box sequence holds its content.
    pub(super) fn container(&self) -> NodeId {
        self.container
    }

    /// The one item of a `::before` / `::after` whose `display` makes it
    /// a flex or grid container (`flex`, `grid`, `inline-flex`,
    /// `inline-grid`; CSS Pseudo 4 §2): the anonymous box wrapping its
    /// generated text (CSS Flexbox §4, CSS Grid 2 §6.1), styled as an
    /// anonymous box inheriting from it. `None` for any other box.
    pub(in crate::render::layout_pass) fn content_item(&self) -> Option<&super::Item> {
        self.inner
            .get_or_init(|| {
                if self.generated.is_none() || !self.style.flow.is_flex_or_grid() {
                    return None;
                }
                let style = Rc::new(crate::style::cascade::anonymous_box_style(&self.style));
                Some(super::Item::Anonymous(Rc::new(Self {
                    container: self.container,
                    content: self.content.clone(),
                    child_range: self.child_range,
                    style,
                    generated: None,
                    in_generated: true,
                    inner: std::cell::OnceCell::new(),
                })))
            })
            .as_ref()
    }

    /// The box's computed style.
    pub(in crate::render::layout_pass) fn style(&self) -> &ComputedStyle {
        &self.style
    }

    /// [`style`](Self::style), shared.
    pub(super) fn style_rc(&self) -> Rc<ComputedStyle> {
        self.style.clone()
    }

    /// The box's padding plus border on each side (padding percentages
    /// against the containing block's width `cb_width`, CSS Box 3 §4.2).
    /// A text run's anonymous box has none.
    fn edges(&self, cb_width: u16) -> Edges {
        let (p, b) = (&self.style.padding, &self.style.border);
        Edges {
            top: p.top.resolve(cb_width).saturating_add(b.top.cells()),
            right: p.right.resolve(cb_width).saturating_add(b.right.cells()),
            bottom: p.bottom.resolve(cb_width).saturating_add(b.bottom.cells()),
            left: p.left.resolve(cb_width).saturating_add(b.left.cells()),
        }
    }

    /// The content box of the border box `rect`.
    pub(in crate::render::layout_pass) fn content_rect(
        &self,
        rect: LayoutRect,
        cb_width: u16,
    ) -> LayoutRect {
        let e = self.edges(cb_width);
        LayoutRect::new(
            rect.x + i32::from(e.left),
            rect.y + i32::from(e.top),
            rect.width.saturating_sub(e.left.saturating_add(e.right)),
            rect.height.saturating_sub(e.top.saturating_add(e.bottom)),
        )
    }

    /// The content width of a box `width` cells wide (its border box).
    fn inner_width(&self, width: u16, cb_width: u16) -> u16 {
        width.saturating_sub(Sizer::horizontal(&self.style, cb_width).chrome())
    }

    /// The item's content packed `width` cells wide (its content box): a
    /// pseudo-element's text in its own `white-space` and `direction`.
    pub(in crate::render::layout_pass) fn pack(
        &self,
        dom: &Dom<TuiExt>,
        width: u16,
    ) -> InlineLayout {
        if let [BoxItem::Generated(host, slot)] = self.content[..] {
            return crate::render::inline::pack_generated(dom, host, slot, &self.style, width);
        }
        pack_run(
            dom,
            self.container,
            &self.content,
            RunPseudos::default(),
            width,
            // A flex or grid container's anonymous item holds no float:
            // its children are items, which do not float.
            None,
        )
    }

    /// [`pack`](Self::pack), cut by the box's own `line-clamp` (a
    /// generated block container's, CSS Overflow 4 §4;
    /// `line_clamp::clamp_lines`): what it lays out and its block size
    /// and baselines read. Its inline size reads the whole content.
    fn pack_clamped(&self, dom: &Dom<TuiExt>, width: u16) -> InlineLayout {
        let mut il = self.pack(dom, width);
        crate::render::layout_pass::line_clamp::clamp_lines(&mut il, &self.style);
        il
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

    /// The item's border-box content size along `direction` — its
    /// content plus its padding and border: on the inline axis its
    /// max-content width (unwrapped) or min-content width (broken at
    /// every soft wrap opportunity, CSS Sizing 3 §4.1 / §4.2); on the
    /// block axis the rows it packs to at a border-box width of `width`,
    /// or of its own definite `width`.
    pub(in crate::render::layout_pass) fn content_size(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        width: u16,
        max_content: bool,
        cb_width: u16,
    ) -> u16 {
        let chrome = Sizer::along(&self.style, direction, cb_width).chrome();
        // A generated grid container's content is its grid's (CSS Grid 2
        // §5.2). A generated flex container's is its one item's — `flex: 0
        // 1 auto`, no margins, padding or border: its max- and min-content
        // widths the text's, and on its block axis the rows the text packs
        // to at the container's content width, which a row's item (as wide
        // as its text up to that width, never below its min-content) and a
        // column's (stretched to it, or fit-content in it) wrap to alike.
        if self.style.flow == crate::layout::Flow::Grid
            && let Some(item) = self.content_item()
        {
            let cross = match direction {
                Direction::Row => width,
                Direction::Column => self.own_width(cb_width).unwrap_or(width),
            };
            let grid = crate::render::layout_pass::grid::generated_content_size(
                dom,
                item,
                &self.style,
                direction,
                cross,
                cb_width,
                max_content,
            );
            return grid.saturating_add(chrome);
        }
        let content = match direction {
            Direction::Row if max_content => self.widest_line(dom, u16::MAX),
            Direction::Row => self.widest_line(dom, 0),
            Direction::Column => {
                let width = self.inner_width(self.own_width(cb_width).unwrap_or(width), cb_width);
                self.pack_clamped(dom, width).height()
            }
        };
        content.saturating_add(chrome)
    }

    /// Its own definite border-box width, if it declares one.
    fn own_width(&self, cb_width: u16) -> Option<u16> {
        match &self.style.width {
            Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => None,
            size => Sizer::horizontal(&self.style, cb_width).outer_opt(size.cells(Some(cb_width))),
        }
    }

    /// The item's border-box size along `direction` as an intrinsic size
    /// contribution (no container size: a percentage is `auto`): its
    /// declared size, else its content size (`max_content` or
    /// min-content), clamped by its `min-*` / `max-*`.
    pub(in crate::render::layout_pass) fn box_size(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
        max_content: bool,
    ) -> u16 {
        let s = &*self.style;
        let (size, min, max) = match direction {
            Direction::Row => (&s.width, &s.min_width, &s.max_width),
            Direction::Column => (&s.height, &s.min_height, &s.max_height),
        };
        let kw = Keywords::for_run(dom, self, direction, cross_budget, cb_width);
        let available = cross_budget;
        let natural = kw.size(size, None, available).unwrap_or_else(|| {
            self.content_size(dom, direction, cross_budget, max_content, cb_width)
        });
        let clamped = crate::layout::clamp_size(
            natural,
            kw.min(min, None, available),
            kw.max(max, None, available),
        );
        kw.sizer().floor(clamped)
    }

    /// Its first and last content rows (baselines, CSS Box Alignment 3
    /// §9.1) in its border box `width` cells wide: the glyph rows of its
    /// first and last lines, below its top padding and border.
    pub(in crate::render::layout_pass) fn content_rows(
        &self,
        dom: &Dom<TuiExt>,
        width: u16,
        cb_width: u16,
    ) -> Option<(u16, u16)> {
        let top = self.edges(cb_width).top;
        if self.style.flow != crate::layout::Flow::Grid {
            let width = self.inner_width(self.own_width(cb_width).unwrap_or(width), cb_width);
            let (first, last) = self.pack_clamped(dom, width).baselines()?;
            return Some((top + first, top + last));
        }
        let chrome = Sizer::vertical(&self.style, cb_width).chrome();
        let rows = self
            .content_size(dom, Direction::Column, width, true, cb_width)
            .saturating_sub(chrome);
        (rows > 0).then(|| (top, top + rows - 1))
    }

    /// Lay the item out at `rect`, its border box: its content inside its
    /// padding and border ([`Self::lay_out_content`]).
    pub(in crate::render::layout_pass) fn lay_out(
        &self,
        dom: &mut Dom<TuiExt>,
        rect: LayoutRect,
        cb_width: u16,
    ) -> AnonymousIfc {
        let content = self.content_rect(rect, cb_width);
        let (at, lines) = self.lay_out_content(dom, content);
        AnonymousIfc::new(
            at,
            lines,
            self.child_range,
            self.generated
                .map(|(host, slot)| GeneratedBox::new(host, slot, rect)),
        )
    }

    /// Lay its content out in `content`, its content box: the rect its
    /// lines sit at and the lines — packed at its width, or, for a
    /// generated flex or grid container, its one item laid out by flex or
    /// grid layout (CSS Flexbox §9, CSS Grid 2 §11) and its lines at that
    /// item's content box.
    pub(in crate::render::layout_pass) fn lay_out_content(
        &self,
        dom: &mut Dom<TuiExt>,
        content: LayoutRect,
    ) -> (LayoutRect, InlineLayout) {
        let Some(item) = self.content_item() else {
            return (content, self.pack_clamped(dom, content.width));
        };
        let boxes = match self.style.flow {
            crate::layout::Flow::Grid => crate::render::layout_pass::grid::layout_generated_grid(
                dom,
                item,
                content,
                &self.style,
                true,
            ),
            _ => crate::render::layout_pass::flex::layout_flex_children(
                dom,
                std::slice::from_ref(item),
                content,
                &self.style,
            ),
        };
        match boxes.into_iter().next() {
            Some(b) => (b.rect, b.inline_layout),
            None => (content, self.pack(dom, content.width)),
        }
    }
}
