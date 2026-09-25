//! The scroll writer — the single place a scroll offset is set.
//!
//! Every interaction (wheel, scrollbar page / drag, keyboard,
//! drag-autoscroll, caret / node reveal) funnels through
//! [`set_scroll_with`], so the clamp to `[0, content − viewport]` and
//! the `scroll` event dispatch are shared.

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
