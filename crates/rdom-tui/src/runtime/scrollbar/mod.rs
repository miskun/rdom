//! Scroll interaction for scroll containers — scrollbar hit testing
//! and thumb drag, keyboard scrolling, drag-autoscroll, and caret /
//! node reveal.
//!
//! Companion to `render::paint_pass::scrollbar` (which paints the
//! track + thumb). Hit-testing and drag share the same geometry math
//! from that module so click targets match what's rendered.
//!
//! Split by concern; this file keeps the shared [`ScrollAxis`] and the
//! re-exports, so callers keep addressing `runtime::scrollbar::*`:
//!
//! - `hit.rs` — scrollbar hit testing: `hit`, [`ScrollbarHit`],
//!   [`ScrollbarPart`].
//! - `drag.rs` — `mousedown` → page or thumb-drag session
//!   (`handle_mousedown`, `extend_drag`, `end_drag`,
//!   `ScrollbarDrag` on the router).
//! - `keys.rs` — keyboard scrolling of the focused scroll container
//!   (`handle_scroll_key`) and its focus cue ([`SCROLL_FOCUS_ATTR`],
//!   `scroll_focus_target`).
//! - `autoscroll.rs` — edge-zone autoscroll for captured drags
//!   (`resolve_autoscroll_container`, `autoscroll_step_for`,
//!   `autoscroll_step`).
//! - `into_view.rs` — `Element.scrollIntoView` over every scroll
//!   container on the ancestor chain (`scroll_element_into_view`).
//! - `reveal.rs` — scroll the caret or a node's region into view
//!   (`reveal_caret`, `service_caret_reveal`, `scroll_into_view`).
//! - `geometry.rs` — scroll metrics over the scrollport and scroll-container
//!   predicates (`nearest_scroll_container`).
//! - `scroll.rs` — the scroll writers: clamp + `scroll` event.
//! - `painted.rs` — the offsets the last frame painted, so any offset
//!   change repaints (`moved_since_paint`, `note_painted`).
//!
//! Hooks into `router::mouse`:
//!
//! - On `mousedown`: call `hit` to see if the click landed on a
//!   scrollbar. If it did, `handle_mousedown` pages (track click) or
//!   begins a drag (thumb click). Beginning a drag engages pointer
//!   capture so subsequent mousemove/mouseup route back here.
//! - On `mousemove` while `router.scrollbar_drag` is set:
//!   `extend_drag` adjusts the scroll offset proportionally to
//!   the cursor's movement along the track.
//! - On `mouseup`: the router's existing pointer-capture release
//!   auto-triggers. `end_drag` clears the drag record.

mod autoscroll;
mod drag;
mod geometry;
mod hit;
mod into_view;
#[cfg(test)]
mod into_view_tests;
mod keys;
mod painted;
mod reveal;
#[cfg(test)]
mod reveal_tests;
#[cfg(test)]
mod reverse_tests;
#[cfg(test)]
mod rtl_tests;
mod scroll;
#[cfg(test)]
mod scrollport_tests;
pub(crate) mod state;

pub(crate) use autoscroll::{autoscroll_step, autoscroll_step_for, resolve_autoscroll_container};
pub(crate) use drag::{ScrollbarDrag, cancel_drag, end_drag, extend_drag, handle_mousedown};
pub(crate) use hit::hit;
pub use hit::{ScrollbarHit, ScrollbarPart};
pub(crate) use into_view::{inset, outset, scroll_element_into_view};
pub use keys::SCROLL_FOCUS_ATTR;
pub(crate) use keys::{handle_scroll_key, scroll_focus_target};
pub(crate) use painted::{moved_since_paint, note_painted};
pub(crate) use reveal::{reveal_caret, scroll_into_view, service_caret_reveal};
#[cfg(test)]
pub(crate) use scroll::take_queued_scroll_events;
pub(crate) use scroll::{
    WriteKind, fire_queued_scroll_events, scroll_bounds, write_offsets, write_offsets_queued,
};

/// Which scrollbar axis a user is interacting with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

#[cfg(test)]
mod tests;
