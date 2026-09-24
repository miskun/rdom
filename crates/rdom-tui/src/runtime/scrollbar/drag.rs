//! Scrollbar mouse interaction — click-on-track to page, drag the
//! thumb to scroll.
//!
//! Hooks into `router::mouse`:
//!
//! - On `mousedown`: after `hit` found a scrollbar, [`handle_mousedown`]
//!   either pages (track click) or begins a thumb drag. Beginning a
//!   drag engages pointer capture so subsequent mousemove/mouseup
//!   route back here.
//! - On `mousemove` while `router.scrollbar_drag` is set:
//!   [`extend_drag`] adjusts the scroll offset proportionally to the
//!   cursor's movement along the track.
//! - On `mouseup`: the router's existing pointer-capture release
//!   auto-triggers. [`end_drag`] clears the drag record.

use rdom_core::NodeId;

use super::geometry::scroll_metrics;
use super::scroll::set_scroll;
use super::{ScrollAxis, ScrollbarHit, ScrollbarPart};
use crate::TuiDom;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;
use crate::render::paint_pass::scrollbar::thumb_geometry;
use crate::runtime::router::Router;

/// Per-session drag state; lives on `Router` between events.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScrollbarDrag {
    element: NodeId,
    axis: ScrollAxis,
    /// Cursor position along the track at `mousedown`.
    initial_cursor: u16,
    /// Scroll offset at `mousedown`.
    initial_scroll: usize,
}

/// Handle a `mousedown` on a scrollbar. Routes to page (for track
/// clicks) or starts a drag (for thumb clicks). Returns `true`
/// when the scrollbar consumed the event — caller should then
/// skip downstream default actions like focus-on-click.
pub(crate) fn handle_mousedown(router: &mut Router, dom: &mut TuiDom, hit: ScrollbarHit) -> bool {
    match hit.part {
        ScrollbarPart::TrackBefore | ScrollbarPart::TrackAfter => {
            page(dom, hit);
            true
        }
        ScrollbarPart::Thumb => {
            begin_drag(router, dom, hit);
            true
        }
    }
}

/// Track click — page scroll by one viewport in the appropriate
/// direction. `TrackBefore` scrolls toward the start; `TrackAfter`
/// toward the end.
fn page(dom: &mut TuiDom, hit: ScrollbarHit) {
    let (viewport, current_scroll) = scroll_metrics(dom, hit.element, hit.axis);
    let sign: i32 = match hit.part {
        ScrollbarPart::TrackBefore => -1,
        ScrollbarPart::TrackAfter => 1,
        ScrollbarPart::Thumb => return,
    };
    let delta = viewport as i32 * sign;
    set_scroll(dom, hit.element, hit.axis, current_scroll as i32 + delta);
}

/// Begin a thumb-drag session. Engages pointer capture on the
/// scrollbar owner so follow-up mousemove/mouseup route there,
/// then records the starting cursor and scroll offset so
/// [`extend_drag`] can compute relative movement.
fn begin_drag(router: &mut Router, dom: &mut TuiDom, hit: ScrollbarHit) {
    let (_, initial_scroll) = scroll_metrics(dom, hit.element, hit.axis);
    let _ = dom.set_pointer_capture(hit.element);
    router.scrollbar_drag = Some(ScrollbarDrag {
        element: hit.element,
        axis: hit.axis,
        initial_cursor: hit.cursor_along_track,
        initial_scroll,
    });
}

/// Extend an in-progress thumb drag to the cursor's current
/// position. Converts the cursor's delta along the track into a
/// scroll delta using the track ↔ content ratio. Returns `true`
/// when the scroll actually changed (caller requests redraw).
pub(crate) fn extend_drag(router: &Router, dom: &mut TuiDom, mouse_x: u16, mouse_y: u16) -> bool {
    let Some(drag) = router.scrollbar_drag else {
        return false;
    };
    let ext = match dom.node(drag.element).tui_ext() {
        Some(e) => e,
        None => return false,
    };
    // Drag math (cursor-delta → scroll-delta) reads from the padding-
    // box per CSS Overflow 3 §3; the track lives in the padding-box,
    // not `content_layout`.
    let border = dom
        .node(drag.element)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let content = rdom_style::layout::compute_padding_box(ext.layout, border);
    let (viewport, content_size, track_len) = match drag.axis {
        ScrollAxis::Vertical => {
            let x_reserves = dom
                .node(drag.element)
                .computed()
                .is_some_and(|c| matches!(c.overflow_x, Overflow::Scroll | Overflow::Auto));
            let adj = if x_reserves { 1 } else { 0 };
            (
                content.height as usize,
                ext.scroll_content_height,
                content.height.saturating_sub(adj),
            )
        }
        ScrollAxis::Horizontal => {
            let y_reserves = dom
                .node(drag.element)
                .computed()
                .is_some_and(|c| matches!(c.overflow_y, Overflow::Scroll | Overflow::Auto));
            let adj = if y_reserves { 1 } else { 0 };
            (
                content.width as usize,
                ext.scroll_content_width,
                content.width.saturating_sub(adj),
            )
        }
    };

    let cursor_now = match drag.axis {
        ScrollAxis::Vertical => mouse_y as i32 - content.y,
        ScrollAxis::Horizontal => mouse_x as i32 - content.x,
    };
    let cursor_delta = cursor_now - drag.initial_cursor as i32;
    let travel = content_size.saturating_sub(viewport);
    if travel == 0 || track_len == 0 {
        return false;
    }
    let (thumb_size, _) = thumb_geometry(track_len, viewport, content_size, drag.initial_scroll);
    let track_travel = track_len.saturating_sub(thumb_size) as i32;
    if track_travel == 0 {
        return false;
    }
    let scroll_delta = (cursor_delta as i64 * travel as i64 / track_travel as i64) as i32;
    let new_scroll = (drag.initial_scroll as i32 + scroll_delta).max(0);
    let before = match drag.axis {
        ScrollAxis::Vertical => ext.scroll_y,
        ScrollAxis::Horizontal => ext.scroll_x,
    };
    let actually_set = set_scroll(dom, drag.element, drag.axis, new_scroll);
    actually_set != before
}

/// Clear the drag record. Pointer capture is released by the
/// router's existing mouseup path (browser-faithful auto-release).
pub(crate) fn end_drag(router: &mut Router) {
    router.scrollbar_drag = None;
}
