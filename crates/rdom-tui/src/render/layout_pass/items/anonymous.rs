//! An anonymous or generated flex or grid item's box (CSS Flexbox §4,
//! CSS Grid 2 §6.1): a run of
//! its container's box sequence with no node of its own — a text run in
//! an anonymous box, or one `::before` / `::after` in a box styled by its
//! computed style — measured and laid out by packing its content inside
//! its padding and border.

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
        }
    }

    /// The flex container.
    pub(super) fn container(&self) -> NodeId {
        self.container
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
    fn content_rect(&self, rect: LayoutRect, cb_width: u16) -> LayoutRect {
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

    /// The item's content packed `width` cells wide (its content box).
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
            // A flex or grid container's anonymous item holds no float:
            // its children are items, which do not float.
            None,
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
        let content = match direction {
            Direction::Row if max_content => self.widest_line(dom, u16::MAX),
            Direction::Row => self.widest_line(dom, 0),
            Direction::Column => {
                let own = match &self.style.width {
                    Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => None,
                    size => Sizer::horizontal(&self.style, cb_width)
                        .outer_opt(size.cells(Some(cb_width))),
                };
                let width = self.inner_width(own.unwrap_or(width), cb_width);
                self.pack(dom, width).height()
            }
        };
        content.saturating_add(chrome)
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
    /// §9.1) in its border box `width` cells wide: its first and last
    /// lines, below its top padding and border.
    pub(in crate::render::layout_pass) fn content_rows(
        &self,
        dom: &Dom<TuiExt>,
        width: u16,
        cb_width: u16,
    ) -> Option<(u16, u16)> {
        let top = self.edges(cb_width).top;
        let rows = self.pack(dom, self.inner_width(width, cb_width)).height();
        (rows > 0).then(|| (top, top + rows - 1))
    }

    /// Lay the item out at `rect`, its border box: its content packed at
    /// its content width, inside its padding and border.
    pub(in crate::render::layout_pass) fn lay_out(
        &self,
        dom: &Dom<TuiExt>,
        rect: LayoutRect,
        cb_width: u16,
    ) -> AnonymousIfc {
        let content = self.content_rect(rect, cb_width);
        AnonymousIfc::new(
            content,
            self.pack(dom, content.width),
            self.child_range,
            self.generated
                .map(|(host, slot)| GeneratedBox::new(host, slot, rect)),
        )
    }
}
