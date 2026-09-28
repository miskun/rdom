//! The scroll writers — the places a scroll offset is set.
//!
//! Scrollbar page / drag, instant keyboard scrolling, drag-autoscroll
//! and caret / node reveal funnel through [`set_scroll_with`] (one
//! axis); the programmatic scroll API and the smooth-scroll steps
//! through [`write_offsets`] (both axes). Both share the clamp to
//! `[0, content − viewport]` and the `scroll` event dispatch. An
//! instant write through [`set_scroll_with`] aborts the box's smooth
//! scroll in flight (CSSOM View "perform a scroll", step 1).

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::node::TuiNodeExt;

/// Set the scroll offset for `element` on `axis`, clamped to
/// `[0, content - viewport]`. Returns the clamped value actually
/// written. Viewport = padding-box per CSS Overflow 3 §3.
pub(super) fn set_scroll(dom: &mut TuiDom, element: NodeId, axis: ScrollAxis, value: i32) -> usize {
    set_scroll_with(dom, element, axis, value, ClampTo::CurrentExtent)
}

/// How `set_scroll_with` bounds the requested offset.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ClampTo {
    /// `[0, content − viewport]` against the extent the last layout
    /// recorded. Wheel, scrollbar drag, programmatic writes.
    CurrentExtent,
    /// `[0, ∞)` — the extent recorded by the last layout is stale
    /// (an edit just added a line) and the next layout's
    /// `clamp_scroll_offset` settles the true maximum. Caret reveal.
    NextLayout,
}

pub(super) fn set_scroll_with(
    dom: &mut TuiDom,
    element: NodeId,
    axis: ScrollAxis,
    value: i32,
    clamp: ClampTo,
) -> usize {
    crate::runtime::smooth_scroll::abort(dom, element);
    let (viewport, content_size) = {
        let ext = match dom.node(element).tui_ext() {
            Some(e) => e,
            None => return 0,
        };
        let border = dom
            .node(element)
            .computed()
            .map(|c| c.border)
            .unwrap_or_default();
        let pb = crate::layout::compute_padding_box(ext.layout, border);
        match axis {
            ScrollAxis::Vertical => (pb.height as usize, ext.scroll_content_height),
            ScrollAxis::Horizontal => (pb.width as usize, ext.scroll_content_width),
        }
    };
    let max = content_size.saturating_sub(viewport) as i32;
    let clamped = match clamp {
        ClampTo::CurrentExtent => value.clamp(0, max),
        ClampTo::NextLayout => value.max(0),
    } as usize;
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
        // M5 D5: scrollbar drag dispatches `scroll` like wheel +
        // programmatic mutation. Only fires when the offset
        // actually moved (dragging at the rail end is a no-op).
        // `scroll`: bubbles, NOT cancelable per HTML.
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        let _ = crate::TuiDispatchExt::dispatch_tui_event(dom, element, &mut tui);
    }
    clamped
}

/// The largest legal `(scroll_left, scroll_top)` of `element` against
/// the extent the last layout recorded; `None` for a non-element.
pub(crate) fn max_offsets(dom: &TuiDom, element: NodeId) -> Option<(i32, i32)> {
    let ext = dom.node(element).tui_ext()?;
    let border = dom
        .node(element)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    Some((
        (ext.scroll_content_width as i32 - pb.width as i32).max(0),
        (ext.scroll_content_height as i32 - pb.height as i32).max(0),
    ))
}

/// Clamp `(x, y)` to `[0, max_offsets]` and write both offsets of
/// `element`, firing one `scroll` event when either moved. Does not
/// touch a smooth scroll in flight — the caller decides (the
/// programmatic API aborts it first, a smooth-scroll step is it).
/// Returns whether an offset moved.
///
/// Viewport is the **padding-box** dimensions, NOT `content_layout`:
/// CSSOM View clamps against the scrollport, which CSS Overflow 3 §3
/// places at the padding-box edge.
pub(crate) fn write_offsets(dom: &mut TuiDom, element: NodeId, x: i32, y: i32) -> bool {
    let Some((max_x, max_y)) = max_offsets(dom, element) else {
        return false;
    };
    let (x, y) = (x.clamp(0, max_x) as usize, y.clamp(0, max_y) as usize);
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
        // `scroll`: bubbles, NOT cancelable per HTML.
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        // `element` held the offsets just written, so it is live.
        let live = crate::tui_event::dispatch_to_live(dom, element, &mut tui);
        debug_assert!(live, "a scroll container that just scrolled is a live node");
    }
    changed
}
