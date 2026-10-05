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

/// Set the scroll offset for `element` on `axis`, clamped to its
/// legal range ([`ScrollBounds`]). Returns the clamped value actually
/// written.
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

/// The legal scroll offsets of a scroll container: layout's one answer
/// (`layout_pass::scrollport`).
pub(crate) use crate::render::layout_pass::scroll_bounds;

/// Clamp `(x, y)` to [`scroll_bounds`] and write both offsets of
/// `element`, firing one `scroll` event when either moved. Does not
/// touch a smooth scroll in flight — the caller decides (the
/// programmatic API aborts it first, a smooth-scroll step is it).
/// Returns whether an offset moved.
///
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
