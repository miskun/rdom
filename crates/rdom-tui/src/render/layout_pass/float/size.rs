//! A float's margin box (CSS 2.1 §10.3.5 / §10.6.7): its used width —
//! a declared one, else shrink-to-fit — and height, and its margins
//! (`auto` margins are 0).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, IntrinsicSize, LayoutRect, Size, clamp_size};
use crate::render::layout_pass::intrinsic::{Keywords, intrinsic_size};
use crate::style::ComputedStyle;

/// A float's border box size and margins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub(crate) fn of(dom: &Dom<TuiExt>, id: NodeId, cb_width: u16) -> Self {
        let c = dom
            .node(id)
            .ext()
            .and_then(|e| e.computed.clone())
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let m = |v: &crate::layout::MarginValue| i32::from(v.resolve(cb_width));
        let (ml, mt, mr, mb) = (
            m(&c.margin.left),
            m(&c.margin.top),
            m(&c.margin.right),
            m(&c.margin.bottom),
        );
        let available = (i32::from(cb_width) - ml - mr).clamp(0, i32::from(u16::MAX)) as u16;
        let kw = Keywords::new(dom, id, &c, Direction::Row, 0, cb_width);
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
        let height = intrinsic_size(dom, id, Direction::Column, width, cb_width);
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
pub(crate) fn outer_contribution(dom: &Dom<TuiExt>, id: NodeId, max_content: bool) -> u16 {
    let inner = crate::render::layout_pass::intrinsic::contribution(
        dom,
        id,
        Direction::Row,
        0,
        0,
        max_content,
    );
    let margins = dom
        .node(id)
        .ext()
        .and_then(|e| e.computed.as_deref())
        .map_or(0, |c| {
            i32::from(c.margin.left.resolve(0)) + i32::from(c.margin.right.resolve(0))
        });
    (i32::from(inner) + margins).clamp(0, i32::from(u16::MAX)) as u16
}
