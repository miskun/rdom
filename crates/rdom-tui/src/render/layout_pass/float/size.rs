//! A float's margin box (CSS 2.1 §10.3.5 / §10.6.7): its used width —
//! a declared one, else shrink-to-fit — and height, and its margins
//! (`auto` margins are 0).

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::layout::{Direction, IntrinsicSize, LayoutRect, Size, clamp_size};
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::items::Item;

/// A float's border box size and margins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FloatBox {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) margin_left: i32,
    pub(crate) margin_top: i32,
    pub(crate) margin_right: i32,
    pub(crate) margin_bottom: i32,
}

impl FloatBox {
    /// `id`'s box in a containing block `cb_width` cells wide: CSS 2.1
    /// §10.3.5 — an `auto` width is shrink-to-fit, `min(max(min-content,
    /// available), max-content)` with the available width the containing
    /// block's less the margins (CSS Sizing 3's `fit-content`), a
    /// declared one as `box-sizing` measures it, either clamped by
    /// `min-width` / `max-width`; §10.6.7 — an `auto` height is the
    /// content's at that width. Margins resolve against the containing
    /// block's width (§8.3), `auto` as 0.
    ///
    /// A `::before` / `::after` is sized as its own box
    /// (`items::AnonymousItem::pseudo`), by the same rules.
    pub(crate) fn of(dom: &Dom<TuiExt>, item: BoxItem, cb_width: u16) -> Self {
        let Some(item) = Item::of_box(dom, item) else {
            return Self::default();
        };
        let c = item.computed(dom);
        let m = |v: &crate::layout::MarginValue| i32::from(v.resolve(cb_width));
        let (ml, mt, mr, mb) = (
            m(&c.margin.left),
            m(&c.margin.top),
            m(&c.margin.right),
            m(&c.margin.bottom),
        );
        let available = (i32::from(cb_width) - ml - mr).clamp(0, i32::from(u16::MAX)) as u16;
        let kw = item.keywords(dom, &c, Direction::Row, 0, cb_width);
        let fit = || kw.keyword(&IntrinsicSize::FitContent, Some(cb_width), available);
        let width = match &c.width {
            Size::Auto | Size::Flex(_) => fit(),
            declared => kw
                .size(declared, Some(cb_width), available)
                .unwrap_or_else(fit),
        };
        let width = kw.sizer().floor(clamp_size(
            width,
            kw.min(&c.min_width, Some(cb_width), available),
            kw.max(&c.max_width, Some(cb_width), available),
        ));
        // The box's height contribution at that width: a declared height,
        // else the content's, clamped by `min-height` / `max-height`.
        let height = item.contribution(dom, Direction::Column, width, cb_width, true);
        Self {
            width,
            height,
            margin_left: ml,
            margin_top: mt,
            margin_right: mr,
            margin_bottom: mb,
        }
    }

    /// The margin box's width (0 when negative margins outweigh it).
    pub(crate) fn outer_width(&self) -> u16 {
        (i32::from(self.width) + self.margin_left + self.margin_right).clamp(0, i32::from(u16::MAX))
            as u16
    }

    /// The margin box's height.
    pub(crate) fn outer_height(&self) -> u16 {
        (i32::from(self.height) + self.margin_top + self.margin_bottom)
            .clamp(0, i32::from(u16::MAX)) as u16
    }

    /// The border box of the float whose margin box's top-left corner is
    /// `(left, top)`.
    pub(crate) fn border_box(&self, left: i32, top: i32) -> LayoutRect {
        LayoutRect::new(
            left + self.margin_left,
            top + self.margin_top,
            self.width,
            self.height,
        )
    }
}

/// `id`'s margin-box contribution to an intrinsic inline size (CSS
/// Sizing 3 §5.2): its max-content contribution when `max_content`, else
/// its min-content one, plus its margins — percentages against no basis
/// (§5.2.1).
pub(crate) fn outer_contribution(dom: &Dom<TuiExt>, item: BoxItem, max_content: bool) -> u16 {
    let Some(item) = Item::of_box(dom, item) else {
        return 0;
    };
    let inner = item.contribution(dom, Direction::Row, 0, 0, max_content);
    let c = item.computed(dom);
    let margins = i32::from(c.margin.left.resolve(0)) + i32::from(c.margin.right.resolve(0));
    (i32::from(inner) + margins).clamp(0, i32::from(u16::MAX)) as u16
}
