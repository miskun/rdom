//! The scroll writers — the places a scroll offset is set.
//!
//! Scrollbar page / drag, instant keyboard scrolling, drag-autoscroll
//! and caret / node reveal funnel through [`set_scroll_with`] (one
//! axis); the programmatic scroll API and the smooth-scroll steps
//! through [`write_offsets`] (both axes). Both share the clamp to
//! [`ScrollBounds`] and the `scroll` event dispatch. An
//! instant write through [`set_scroll_with`] aborts the box's smooth
//! scroll in flight (CSSOM View "perform a scroll", step 1).

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::node::TuiNodeExt;

/// Set the scroll offset for `element` on `axis`, clamped to its
/// legal range ([`ScrollBounds`]). Returns the clamped value actually
/// written. Viewport = padding-box per CSS Overflow 3 §3.
pub(super) fn set_scroll(dom: &mut TuiDom, element: NodeId, axis: ScrollAxis, value: i32) -> i32 {
    set_scroll_with(dom, element, axis, value, ClampTo::CurrentExtent)
}

/// How `set_scroll_with` bounds the requested offset.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ClampTo {
    /// The legal range against the extent the last layout recorded.
    /// Wheel, scrollbar drag, programmatic writes.
    CurrentExtent,
    /// Only the origin side — `[0, ∞)`, or `(-∞, 0]` for an `rtl`
    /// box's `scrollLeft` — the extent recorded by the last layout is
    /// stale (an edit just added a line) and the next layout's
    /// `clamp_scroll_offset` settles the true extreme. Caret reveal.
    NextLayout,
}

pub(super) fn set_scroll_with(
    dom: &mut TuiDom,
    element: NodeId,
    axis: ScrollAxis,
    value: i32,
    clamp: ClampTo,
) -> i32 {
    crate::runtime::smooth_scroll::abort(dom, element);
    let Some(bounds) = scroll_bounds(dom, element) else {
        return 0;
    };
    let (min, max, origin_at_end) = match axis {
        ScrollAxis::Vertical => (bounds.min_y, bounds.max_y, bounds.origin_at_end.1),
        ScrollAxis::Horizontal => (bounds.min_x, bounds.max_x, bounds.origin_at_end.0),
    };
    let clamped = match clamp {
        ClampTo::CurrentExtent => value.clamp(min, max),
        // The origin is the bound at 0; the far one waits for layout.
        ClampTo::NextLayout if origin_at_end => value.min(0),
        ClampTo::NextLayout => value.max(0),
    };
    let changed = if let Some(ext) = dom.node_mut(element).ext_mut() {
        match axis {
            ScrollAxis::Vertical => {
                let changed = ext.scroll_y != clamped;
                ext.scroll_y = clamped;
                changed
            }
            ScrollAxis::Horizontal => {
                let changed = ext.scroll_x != clamped;
                ext.scroll_x = clamped;
                changed
            }
        }
    } else {
        false
    };
    if changed {
        crate::runtime::state_writes::note();
        // M5 D5: scrollbar drag dispatches `scroll` like wheel +
        // programmatic mutation. Only fires when the offset
        // actually moved (dragging at the rail end is a no-op).
        // `scroll`: bubbles, NOT cancelable per HTML.
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        crate::tui_event::dispatch_to_live(dom, element, &mut tui);
    }
    clamped
}

/// The legal scroll offsets of a scroll container against the extent
/// the last layout recorded and its padding-box scrollport (CSS
/// Overflow 3 §3): `scrollLeft` in `min_x ..= max_x`, `scrollTop` in
/// `min_y ..= max_y` — `0 ..= overflow`, or `-overflow ..= 0` when the
/// scrolling area origin is the right (bottom) edge: an `rtl` box, a
/// flex container's reversed main axis (`row-reverse`,
/// `column-reverse`) or reversed cross axis (`wrap-reverse`) (CSSOM View §4,
/// `layout_pass::scroll_x_bounds` / `scroll_y_bounds`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScrollBounds {
    pub(crate) min_x: i32,
    pub(crate) max_x: i32,
    pub(crate) min_y: i32,
    pub(crate) max_y: i32,
    /// Whether each axis's scrolling area origin is its right (bottom)
    /// edge, `(horizontal, vertical)` — known from the box, not from the
    /// extent, which is 0 while nothing overflows.
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

/// `element`'s [`ScrollBounds`]; `None` for a non-element.
pub(crate) fn scroll_bounds(dom: &TuiDom, element: NodeId) -> Option<ScrollBounds> {
    let ext = dom.node(element).tui_ext()?;
    let border = dom
        .node(element)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    let (min_x, max_x) =
        crate::render::layout_pass::scroll_x_bounds(dom, element, pb.width as usize);
    let (min_y, max_y) =
        crate::render::layout_pass::scroll_y_bounds(dom, element, pb.height as usize);
    Some(ScrollBounds {
        min_x,
        max_x,
        min_y,
        max_y,
        origin_at_end: crate::render::layout_pass::origin_at_end(dom, element),
    })
}

/// Clamp `(x, y)` to [`scroll_bounds`] and write both offsets of
/// `element`, firing one `scroll` event when either moved. Does not
/// touch a smooth scroll in flight — the caller decides (the
/// programmatic API aborts it first, a smooth-scroll step is it).
/// Returns whether an offset moved.
///
/// Viewport is the **padding-box** dimensions, NOT `content_layout`:
/// CSSOM View clamps against the scrollport, which CSS Overflow 3 §3
/// places at the padding-box edge.
pub(crate) fn write_offsets(dom: &mut TuiDom, element: NodeId, x: i32, y: i32) -> bool {
    let Some(bounds) = scroll_bounds(dom, element) else {
        return false;
    };
    let (x, y) = bounds.clamp(x, y);
    let changed = match dom.node_mut(element).ext_mut() {
        Some(ext) => {
            let changed = (ext.scroll_x, ext.scroll_y) != (x, y);
            ext.scroll_x = x;
            ext.scroll_y = y;
            changed
        }
        None => false,
    };
    if changed {
        crate::runtime::state_writes::note();
        // `scroll`: bubbles, NOT cancelable per HTML.
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        // `element` held the offsets just written, so it is live.
        let live = crate::tui_event::dispatch_to_live(dom, element, &mut tui);
        debug_assert!(live, "a scroll container that just scrolled is a live node");
    }
    changed
}
