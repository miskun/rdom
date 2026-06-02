//! Scrollbar mouse interaction — click-on-track to page, drag
//! the thumb to scroll.
//!
//! Companion to `render::paint_pass::scrollbar` (which paints the
//! track + thumb). Hit-testing and drag share the same geometry
//! math from that module so click targets match what's rendered.
//!
//! Hooks into `router::mouse`:
//!
//! - On `mousedown`: call [`hit`] to see if the click landed on a
//!   scrollbar. If it did, either [`page`] (track click) or
//!   [`begin_drag`] (thumb click). When begin_drag fires, it
//!   engages pointer capture so subsequent mousemove/mouseup
//!   route back here.
//! - On `mousemove` while `router.scrollbar_drag` is set:
//!   [`extend_drag`] adjusts the scroll offset proportionally to
//!   the cursor's movement along the track.
//! - On `mouseup`: the router's existing pointer-capture release
//!   auto-triggers. [`end_drag`] clears the drag record.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::render::paint_pass::scrollbar::{should_paint, thumb_geometry};
use crate::runtime::router::Router;

/// Which scrollbar axis a user is interacting with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

/// What part of a scrollbar got clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarPart {
    /// Mouse on the track above / left of the thumb — page back.
    TrackBefore,
    /// Mouse on the thumb — start a drag.
    Thumb,
    /// Mouse on the track below / right of the thumb — page forward.
    TrackAfter,
}

/// Result of a scrollbar hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarHit {
    pub element: NodeId,
    pub axis: ScrollAxis,
    pub part: ScrollbarPart,
    /// Cursor offset along the scrollbar track in cells (from track
    /// start). Used by `begin_drag` to compute the thumb-relative
    /// anchor so dragging doesn't snap the thumb.
    pub cursor_along_track: u16,
}

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

/// Check whether `(x, y)` lands on a scrollbar rendered for any
/// ancestor starting at `path_inner` (the hit-test path, innermost-
/// first). Returns the first match walking outward — which is the
/// same element the user perceives as the scrollbar owner.
pub(crate) fn hit(dom: &TuiDom, path: &[NodeId], x: u16, y: u16) -> Option<ScrollbarHit> {
    // Walk the path outward — scrollbars belong to the scrollable
    // container, and its content_layout's gutter is OUTSIDE content
    // but INSIDE its outer rect, so the container is the last path
    // element (or one of its ancestors) containing the point.
    for &id in path.iter().rev() {
        if let Some(h) = check_element(dom, id, x, y) {
            return Some(h);
        }
    }
    None
}

fn check_element(dom: &TuiDom, id: NodeId, x: u16, y: u16) -> Option<ScrollbarHit> {
    let ext = dom.node(id).tui_ext()?;
    let content = ext.content_layout;
    let computed = dom.node(id).computed()?;
    // CSS Overflow 3 §3: scrollbar gutter is inside the padding-box.
    // Column position uses `content_layout` (gutter already accounted
    // for by `reserve_scrollbar_gutter`); track extent is clamped to
    // padding-box so a click on a border-row column doesn't register
    // as a scrollbar hit under M5.5b border-collapse.
    let padding_box = rdom_style::layout::compute_padding_box(ext.layout, computed.border);

    let y_reserves = matches!(computed.overflow_y, Overflow::Scroll | Overflow::Auto);
    let x_reserves = matches!(computed.overflow_x, Overflow::Scroll | Overflow::Auto);

    // Vertical scrollbar sits in the column just right of
    // content.x + content.width. Horizontal sits in the row just
    // below content.y + content.height.
    let v_col = content.x + content.width as i32;
    let h_row = content.y + content.height as i32;
    let in_v_col = x as i32 == v_col;
    let in_h_row = y as i32 == h_row;

    // Vertical track spans y in [content.y, content.y + height) clamped
    // to padding-box rows; minus one row for the corner if horizontal
    // also reserves.
    let v_top = content.y.max(padding_box.y);
    let mut v_bottom =
        (content.y + content.height as i32).min(padding_box.y + padding_box.height as i32);
    if x_reserves {
        v_bottom -= 1;
    }
    let in_v_rows = (y as i32) >= v_top && (y as i32) < v_bottom;

    let h_left = content.x.max(padding_box.x);
    let mut h_right =
        (content.x + content.width as i32).min(padding_box.x + padding_box.width as i32);
    if y_reserves {
        h_right -= 1;
    }
    let in_h_cols = (x as i32) >= h_left && (x as i32) < h_right;

    if y_reserves && in_v_col && in_v_rows {
        let track_len = (v_bottom - v_top) as u16;
        let viewport = content.height;
        let content_size = ext.scroll_content_height;
        if !should_paint(computed.overflow_y, viewport as usize, content_size) {
            return None;
        }
        let (thumb_size, thumb_off) =
            thumb_geometry(track_len, viewport as usize, content_size, ext.scroll_y);
        let cursor_along = (y as i32 - v_top) as u16;
        return Some(ScrollbarHit {
            element: id,
            axis: ScrollAxis::Vertical,
            part: classify(cursor_along, thumb_off, thumb_size),
            cursor_along_track: cursor_along,
        });
    }

    if x_reserves && in_h_row && in_h_cols {
        let track_len = (h_right - h_left) as u16;
        let viewport = content.width;
        let content_size = ext.scroll_content_width;
        if !should_paint(computed.overflow_x, viewport as usize, content_size) {
            return None;
        }
        let (thumb_size, thumb_off) =
            thumb_geometry(track_len, viewport as usize, content_size, ext.scroll_x);
        let cursor_along = (x as i32 - h_left) as u16;
        return Some(ScrollbarHit {
            element: id,
            axis: ScrollAxis::Horizontal,
            part: classify(cursor_along, thumb_off, thumb_size),
            cursor_along_track: cursor_along,
        });
    }

    None
}

fn classify(cursor: u16, thumb_off: u16, thumb_size: u16) -> ScrollbarPart {
    if cursor < thumb_off {
        ScrollbarPart::TrackBefore
    } else if cursor < thumb_off + thumb_size {
        ScrollbarPart::Thumb
    } else {
        ScrollbarPart::TrackAfter
    }
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

// ── Helpers ─────────────────────────────────────────────────────────

/// `(viewport_size_in_cells, current_scroll_offset)` for a given
/// element + axis. Viewport = padding-box per CSS Overflow 3 §3.
fn scroll_metrics(dom: &TuiDom, element: NodeId, axis: ScrollAxis) -> (u16, usize) {
    let ext = match dom.node(element).tui_ext() {
        Some(e) => e,
        None => return (0, 0),
    };
    let border = dom
        .node(element)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
    match axis {
        ScrollAxis::Vertical => (pb.height, ext.scroll_y),
        ScrollAxis::Horizontal => (pb.width, ext.scroll_x),
    }
}

/// Set the scroll offset for `element` on `axis`, clamped to
/// `[0, content - viewport]`. Returns the clamped value actually
/// written. Viewport = padding-box per CSS Overflow 3 §3.
/// Scroll the nearest vertically-scrollable ancestor of `node` so the
/// viewport-coord region `reveal` becomes visible. Uses
/// `block: "nearest"` alignment that never hides `reveal`'s TOP edge,
/// so a navigation cursor row stays anchored — the ARIA listbox/tree
/// pattern, and the browser's focus scroll-into-view. Vertical axis
/// only for now; nested scroll containers resolve to the nearest one
/// (sufficient for current widgets — revisit if a nested-scroller case
/// appears).
///
/// Reuses [`set_scroll`], so the clamp to `[0, max]` and the `scroll`
/// event dispatch are shared with wheel / scrollbar interaction.
pub(crate) fn scroll_into_view(dom: &mut TuiDom, node: NodeId, reveal: LayoutRect) {
    let mut cur = dom.node(node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if is_vertical_scroll_container(dom, id) {
            ensure_visible_vertical(dom, id, reveal);
            return;
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
}

/// `true` when `id` clips on the Y axis and has more content than its
/// scrollport can show (i.e. there's somewhere to scroll to).
fn is_vertical_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let Some(ext) = dom.node(id).tui_ext() else {
        return false;
    };
    let overflow_y = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_y)
        .unwrap_or(Overflow::Visible);
    if matches!(overflow_y, Overflow::Visible) {
        return false;
    }
    let border = dom
        .node(id)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
    ext.scroll_content_height > pb.height as usize
}

/// Adjust `container`'s vertical scroll so `reveal` is in the
/// scrollport, anchoring `reveal`'s top edge (never scroll so far down
/// that the top leaves the view).
fn ensure_visible_vertical(dom: &mut TuiDom, container: NodeId, reveal: LayoutRect) {
    let (port_top, port_bottom, cur_scroll) = {
        let Some(ext) = dom.node(container).tui_ext() else {
            return;
        };
        let border = dom
            .node(container)
            .computed()
            .map(|c| c.border)
            .unwrap_or_default();
        let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
        (pb.y, pb.y + pb.height as i32, ext.scroll_y as i32)
    };
    let r_top = reveal.y;
    let r_bottom = reveal.y + reveal.height as i32;
    let delta = if r_top < port_top {
        // Region above the scrollport — scroll up to reveal its top.
        r_top - port_top
    } else if r_bottom > port_bottom {
        // Region below — scroll down just enough, but never past the
        // top edge (so a region taller than the port aligns its top).
        (r_bottom - port_bottom).min(r_top - port_top).max(0)
    } else {
        0
    };
    if delta != 0 {
        set_scroll(dom, container, ScrollAxis::Vertical, cur_scroll + delta);
    }
}

fn set_scroll(dom: &mut TuiDom, element: NodeId, axis: ScrollAxis, value: i32) -> usize {
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
        let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
        match axis {
            ScrollAxis::Vertical => (pb.height as usize, ext.scroll_content_height),
            ScrollAxis::Horizontal => (pb.width as usize, ext.scroll_content_width),
        }
    };
    let max = content_size.saturating_sub(viewport) as i32;
    let clamped = value.clamp(0, max) as usize;
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

/// `true` when `id` clips on the X axis and its content is wider than its
/// scrollport. Horizontal analog of [`is_vertical_scroll_container`].
fn is_horizontal_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let Some(ext) = dom.node(id).tui_ext() else {
        return false;
    };
    let overflow_x = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_x)
        .unwrap_or(Overflow::Visible);
    if matches!(overflow_x, Overflow::Visible) {
        return false;
    }
    let border = dom
        .node(id)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
    ext.scroll_content_width > pb.width as usize
}

/// Keyboard scrolling for a **focused scroll container** (`FOCUS-VOCAB-1`):
/// Arrows / PageUp / PageDown / Home / End / Space scroll the focused element
/// along whichever axes it can scroll. Mirrors the web, where a focused
/// scrollable region is keyboard-scrollable.
///
/// Returns `true` when the key was a scroll key the focused element handles —
/// the caller treats it as a consumed default action (request a redraw, skip
/// focus-nav fallthrough). Keys for a non-scrollable axis return `false` so
/// they fall through to other defaults. Runs *after* the editable-key default,
/// so a focused `<input>`/`<textarea>` still moves its caret with arrows.
pub(crate) fn handle_scroll_key(dom: &mut TuiDom, key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};

    let Some(el) = dom.focused() else {
        return false;
    };
    let vert = is_vertical_scroll_container(dom, el);
    let horiz = is_horizontal_scroll_container(dom, el);
    if !vert && !horiz {
        return false;
    }

    let (vh, vscroll) = scroll_metrics(dom, el, ScrollAxis::Vertical);
    let page = (vh as i32).max(1);

    match key.code {
        KeyCode::Down if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, vscroll as i32 + 1);
            true
        }
        KeyCode::Up if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, vscroll as i32 - 1);
            true
        }
        KeyCode::PageDown if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, vscroll as i32 + page);
            true
        }
        KeyCode::PageUp if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, vscroll as i32 - page);
            true
        }
        KeyCode::Char(' ') if vert => {
            let dir = if key.modifiers.contains(KeyModifiers::SHIFT) {
                -page
            } else {
                page
            };
            set_scroll(dom, el, ScrollAxis::Vertical, vscroll as i32 + dir);
            true
        }
        KeyCode::Home if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, 0);
            true
        }
        KeyCode::End if vert => {
            set_scroll(dom, el, ScrollAxis::Vertical, i32::MAX);
            true
        }
        KeyCode::Right if horiz => {
            let (_, hscroll) = scroll_metrics(dom, el, ScrollAxis::Horizontal);
            set_scroll(dom, el, ScrollAxis::Horizontal, hscroll as i32 + 1);
            true
        }
        KeyCode::Left if horiz => {
            let (_, hscroll) = scroll_metrics(dom, el, ScrollAxis::Horizontal);
            set_scroll(dom, el, ScrollAxis::Horizontal, hscroll as i32 - 1);
            true
        }
        _ => false,
    }
}

#[allow(dead_code)]
fn _layout_rect_unused(_: LayoutRect) {}

#[cfg(test)]
mod tests;
