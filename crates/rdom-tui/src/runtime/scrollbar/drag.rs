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
//! - On the next `mousedown`, when that mouseup was lost:
//!   [`cancel_drag`] clears the record and drops the capture the drag
//!   took, so the new press never continues the old drag.

use rdom_core::NodeId;

use super::geometry::scroll_metrics;
use super::scroll::set_scroll;
use super::{ScrollAxis, ScrollbarHit, ScrollbarPart};
use crate::TuiDom;
use crate::render::paint_pass::scrollbar::thumb_geometry;
use crate::runtime::router::Router;

/// Per-session drag state; lives on `Router` between events.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScrollbarDrag {
    element: NodeId,
    axis: ScrollAxis,
    /// Cursor position along the track at `mousedown`.
    initial_cursor: u16,
    /// Scroll offset at `mousedown` (`scrollTop` / `scrollLeft`).
    initial_scroll: i32,
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
    // A page by the track is a scroll by a delta: a snap container rests
    // at the snap position in its direction (CSS Scroll Snap 1 §6.2).
    let Some(from) = dom
        .node(hit.element)
        .ext()
        .map(|e| (e.scroll_x, e.scroll_y))
    else {
        return;
    };
    let to = match hit.axis {
        ScrollAxis::Vertical => (from.0, current_scroll + delta),
        ScrollAxis::Horizontal => (current_scroll + delta, from.1),
    };
    let to = crate::runtime::scroll_snap::snap(
        dom,
        hit.element,
        to,
        crate::runtime::scroll_snap::Motion::By { from },
    );
    let value = match hit.axis {
        ScrollAxis::Vertical => to.1,
        ScrollAxis::Horizontal => to.0,
    };
    set_scroll(dom, hit.element, hit.axis, value);
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
    // The track the drag started on (`paint_pass::scrollbar::tracks`):
    // cursor travel along it maps to scroll travel over the area.
    let (vertical, horizontal) = crate::render::paint_pass::scrollbar::tracks(dom, drag.element);
    let (track, cursor_now) = match drag.axis {
        ScrollAxis::Vertical => (vertical, mouse_y as i32),
        ScrollAxis::Horizontal => (horizontal, mouse_x as i32),
    };
    let Some(track) = track else {
        return false;
    };
    let cursor_now = cursor_now - track.start;
    let (viewport, content_size, track_len) = (track.viewport, track.content, track.len);
    let cursor_delta = cursor_now - drag.initial_cursor as i32;
    let travel = content_size.saturating_sub(viewport);
    if travel == 0 || track_len == 0 {
        return false;
    }
    // The thumb's size; its position does not matter to the delta.
    let (thumb_size, _) = thumb_geometry(track_len, viewport, content_size, 0);
    let track_travel = track_len.saturating_sub(thumb_size) as i32;
    if track_travel == 0 {
        return false;
    }
    let scroll_delta = (cursor_delta as i64 * travel as i64 / track_travel as i64) as i32;
    // `set_scroll` clamps to the legal range (negative `scrollLeft`
    // included, for an `rtl` box).
    let new_scroll = drag.initial_scroll + scroll_delta;
    let before = scroll_metrics(dom, drag.element, drag.axis).1;
    let actually_set = set_scroll(dom, drag.element, drag.axis, new_scroll);
    actually_set != before
}

/// Clear the drag record. Pointer capture is released by the
/// router's existing mouseup path (browser-faithful auto-release).
pub(crate) fn end_drag(router: &mut Router, dom: &mut TuiDom) {
    // The drag's scroll ends: a snap container comes to rest at the snap
    // position nearest where it was let go (CSS Scroll Snap 1 §6.2).
    if let Some(drag) = router.scrollbar_drag.take()
        && let Some(at) = dom
            .node(drag.element)
            .ext()
            .map(|e| (e.scroll_x, e.scroll_y))
    {
        let to = crate::runtime::scroll_snap::snap(
            dom,
            drag.element,
            at,
            crate::runtime::scroll_snap::Motion::To,
        );
        if to != at {
            super::scroll::write_offsets(dom, drag.element, to.0, to.1);
        }
    }
}

/// End a thumb drag whose `mouseup` never arrived (the button was
/// released outside the terminal window), releasing the pointer capture
/// [`begin_drag`] took. Called by every left `mousedown` before it does
/// anything else, so a new press never continues the old drag
/// (P6G-PRESS-RESET-2).
///
/// Only rdom's own capture is dropped: the capture is released only
/// while it is still on the drag's scrollbar owner. A capture an author
/// took is theirs, and like a browser's it lasts until the `mouseup`
/// (or the next button-less move, the runtime's stand-in for
/// `pointercancel`).
pub(crate) fn cancel_drag(router: &mut Router, dom: &mut TuiDom) {
    if let Some(drag) = router.scrollbar_drag.take()
        && dom.pointer_capture() == Some(drag.element)
    {
        dom.release_pointer_capture();
    }
}
