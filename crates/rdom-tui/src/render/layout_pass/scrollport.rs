//! A scroll container's scrollport and its scroll range — the one
//! answer every reader takes (layout's clamp, the runtime's wheel, keys,
//! scrollbar, `scrollTo`, `scrollIntoView`, focus scrolling, snapping,
//! sticky positioning, containing blocks, paint and hit-testing).
//!
//! Three boxes, one model (CSS Overflow 3, CSSOM View §4):
//!
//! - **The scrollport** ([`scrollport`]): the padding box less the
//!   scrollbar gutters layout reserved (§5.2: the gutter lies "between
//!   the inner border edge and the outer padding edge"). It is the
//!   visible part of the scrolled content, and the region a scroll
//!   container's content is clipped to ([`super::ClipEdges`]), the
//!   containing block it gives the boxes it contains (before its scroll
//!   offset), the sticky view rectangle's and the snapport's base. The
//!   border is always outside it, `border-collapse` or not. The content
//!   box is the scrollport inset by the padding.
//! - **The scrollable overflow area** (`TuiExt::scroll_content_width` /
//!   `_height`, recorded by `scroll_extent`): the scrollport ∪ the
//!   content, the in-flow content extended by the end padding (§2.2),
//!   measured from the scrolling area origin — content on the origin's
//!   far side of the scrollport is unreachable and not in it.
//! - **The scroll range** ([`scroll_bounds`]): the area's size less the
//!   scrollport's on each axis — `0 ..= range`, or `-range ..= 0` where
//!   the origin is the right (bottom) edge.
//!
//! The gutters are the ones layout reserved (`ScrollState::gutters`),
//! so a reader never re-derives whether an `auto` bar is shown.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::style::ComputedStyle;

/// `ext`'s scrollport for its style `c`: the padding box less the
/// gutters layout reserved — a table's table box's (CSS 2.1 §17.4,
/// `TuiExt::border_box`), its captions outside it.
pub(crate) fn scrollport_of(ext: &TuiExt, c: &ComputedStyle) -> LayoutRect {
    let pb = super::geometry::compute_padding_box(ext.border_box(), c.border);
    let g = crate::runtime::scrollbar::state::gutters(ext);
    LayoutRect::new(
        pb.x + i32::from(g.left),
        pb.y,
        pb.width.saturating_sub(g.columns()),
        pb.height.saturating_sub(g.bottom),
    )
}

/// `id`'s scrollport; its border box while it has no computed style
/// (not cascaded yet: no border, no gutters); `None` for a non-element.
pub(crate) fn scrollport(dom: &Dom<TuiExt>, id: NodeId) -> Option<LayoutRect> {
    let ext = dom.node(id).ext()?;
    Some(match ext.computed.as_deref() {
        Some(c) => scrollport_of(ext, c),
        None => ext.border_box(),
    })
}

/// The legal scroll offsets of a box against the area the last layout
/// recorded and its [`scrollport`] (CSSOM View §4): `scrollLeft` in
/// `min_x ..= max_x`, `scrollTop` in `min_y ..= max_y` — `0 ..= range`,
/// or `-range ..= 0` when the scrolling area origin is the right
/// (bottom) edge: an `rtl` box, a flex container's reversed main axis
/// (`row-reverse`, `column-reverse`) or reversed cross axis
/// (`wrap-reverse`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScrollBounds {
    pub(crate) min_x: i32,
    pub(crate) max_x: i32,
    pub(crate) min_y: i32,
    pub(crate) max_y: i32,
    /// Whether each axis's scrolling area origin is its right (bottom)
    /// edge, `(horizontal, vertical)` — known from the box, not from the
    /// extent, which is the scrollport's while nothing overflows.
    pub(crate) origin_at_end: (bool, bool),
}

impl ScrollBounds {
    /// `(x, y)` clamped into the bounds.
    pub(crate) fn clamp(&self, x: i32, y: i32) -> (i32, i32) {
        (
            x.clamp(self.min_x, self.max_x),
            y.clamp(self.min_y, self.max_y),
        )
    }
}

/// How far the area reaches past the scrollport on each axis, `(x, y)`.
pub(crate) fn scroll_range(dom: &Dom<TuiExt>, id: NodeId) -> (usize, usize) {
    let (Some(ext), Some(port)) = (dom.node(id).ext(), scrollport(dom, id)) else {
        return (0, 0);
    };
    range_over(ext, port)
}

/// [`scroll_range`] from the box's ext and style.
pub(crate) fn range_of(ext: &TuiExt, c: &ComputedStyle) -> (usize, usize) {
    range_over(ext, scrollport_of(ext, c))
}

/// How far `ext`'s area reaches past `port`.
fn range_over(ext: &TuiExt, port: LayoutRect) -> (usize, usize) {
    (
        ext.scroll_content_width
            .saturating_sub(usize::from(port.width)),
        ext.scroll_content_height
            .saturating_sub(usize::from(port.height)),
    )
}

/// `id`'s [`ScrollBounds`]; `None` for a non-element.
pub(crate) fn scroll_bounds(dom: &Dom<TuiExt>, id: NodeId) -> Option<ScrollBounds> {
    dom.node(id).ext()?;
    let (range_x, range_y) = scroll_range(dom, id);
    let origin_at_end = super::scroll_extent::origin_at_end(dom, id);
    let (min_x, max_x) = bounds(range_x, origin_at_end.0);
    let (min_y, max_y) = bounds(range_y, origin_at_end.1);
    Some(ScrollBounds {
        min_x,
        max_x,
        min_y,
        max_y,
        origin_at_end,
    })
}

/// The legal offsets on one axis: `0 ..= range` from an origin at the
/// left / top edge, `-range ..= 0` from one at the right / bottom.
fn bounds(range: usize, origin_at_end: bool) -> (i32, i32) {
    let range = i32::try_from(range).unwrap_or(i32::MAX);
    if origin_at_end {
        (-range, 0)
    } else {
        (0, range)
    }
}

/// How far `id`'s scrollport sits from the start (left, top) of its
/// scrollable overflow area, `(x, y)`: the offset less its minimum.
/// Physical and never negative — where a scrollbar thumb is drawn.
pub(crate) fn offset_from_area_start(dom: &Dom<TuiExt>, id: NodeId) -> (usize, usize) {
    let Some(b) = scroll_bounds(dom, id) else {
        return (0, 0);
    };
    let (sx, sy) = dom
        .node(id)
        .ext()
        .map_or((0, 0), |e| (e.scroll_x, e.scroll_y));
    let from = |v: i32, min: i32| usize::try_from(v.saturating_sub(min)).unwrap_or(0);
    (from(sx, b.min_x), from(sy, b.min_y))
}

/// Whether `id` has somewhere to scroll on each axis, `(x, y)`: its area
/// reaches past its scrollport.
pub(crate) fn overflows(dom: &Dom<TuiExt>, id: NodeId) -> (bool, bool) {
    let (x, y) = scroll_range(dom, id);
    (x > 0, y > 0)
}
