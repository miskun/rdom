//! Keyboard scrolling of the focused scroll container (`FOCUS-VOCAB-1`)
//! and the focus-cue attribute that marks which container the keys
//! act on.

use rdom_core::NodeId;

use super::ScrollAxis;
use super::geometry::{
    is_horizontal_scroll_container, is_vertical_scroll_container, nearest_scroll_container,
    scroll_metrics,
};
use super::scroll::set_scroll;
use crate::TuiDom;

/// Attribute the runtime keeps on the scroll container the keyboard
/// scrolls (see `scroll_focus_target`); the UA sheet colors that
/// container's scrollbar thumb through it. Author rules may match it
/// too. Written with `set_attribute` / `remove_attribute` before the
/// frame's cascade, so mutation observers see the moves.
pub const SCROLL_FOCUS_ATTR: &str = "data-rdom-scroll-focus";

/// The scroll container the focus cue belongs on: the nearest scroll
/// container of the focused element, per the last layout. `None` when
/// nothing is focused or no ancestor scrolls.
pub(crate) fn scroll_focus_target(dom: &TuiDom) -> Option<NodeId> {
    nearest_scroll_container(dom, dom.focused()?)
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

    let Some(focused) = dom.focused() else {
        return false;
    };
    // Scroll the nearest scrollable ancestor of the focused element (itself
    // included). So a focused `<input>` inside a scroll pane still pages the
    // pane (the focused element isn't a scroll container, an ancestor is) —
    // the web's "scroll keys act on the scrolling element the focus is in".
    let Some(el) = nearest_scroll_container(dom, focused) else {
        return false;
    };
    let vert = is_vertical_scroll_container(dom, el);
    let horiz = is_horizontal_scroll_container(dom, el);

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
