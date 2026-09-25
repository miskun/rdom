//! `user-select` behaviors — what `UserSelect::{None, All, Contain}`
//! do to mouse drag, keyboard extension and clipboard serialization.
//!
//! The CSS property + cascade rung lives in `rdom-style`; its **used
//! value** is resolved in [`crate::style::user_select`] (re-exported
//! here), a pure function of the computed style that paint and
//! hit-testing share. This module owns the runtime *behavior* it
//! controls.
//!
//! ## Behaviors
//!
//! - **`None`** — unselectable ([`is_unselectable`]). Drag, keyboard
//!   extension, the highlight and clipboard serialize all skip it.
//! - **`All`** — the host ([`all_host`]: the outermost ancestor whose
//!   used value is `all`) is selected atomically. A click anywhere
//!   inside expands the selection to span the host's full text
//!   ([`span_all_text`]); subsequent drag-extend and keyboard-extend
//!   are suppressed while the focus remains inside the host.
//! - **`Contain`** — selections started inside the host
//!   ([`contain_host`]: the nearest ancestor whose used value is
//!   `contain`, i.e. that declares it or is an editable `auto`
//!   element) cannot escape. Drag-extend whose hit position falls
//!   outside the host clamps to the in-host line nearest the
//!   pointer's row ([`clamp_to_contain_host`]).
//!
//! `Text` is a no-op at the policy layer — the default "selectable"
//! state the drag pipeline assumes.

use crossterm::event::MouseEvent;

use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::node::{TuiNodeExt, first_text_descendant, last_text_descendant, text_len};
pub(crate) use crate::style::user_select::{
    all_host, contain_host, is_unselectable, resolve_child, used_value,
};

/// Build a Selection spanning every character of `host`'s text
/// subtree. Anchor = first text node start, focus = last text node
/// end. Returns `None` when no text exists.
pub(crate) fn span_all_text(dom: &TuiDom, host: NodeId) -> Option<Selection> {
    let first = first_text_descendant(dom, host)?;
    let last = last_text_descendant(dom, host).unwrap_or(first);
    let end = text_len(dom, last);
    Some(Selection::new(
        Position::new(first, 0),
        Position::new(last, end),
    ))
}

/// Clamp focus to the nearest in-host position for
/// `user-select: contain`: the in-host inline flow nearest to the
/// pointer's row, resolved like any hit on that flow (the pointer's
/// column on the line, past its end when beside it, the last line's
/// end when below), so a drag out of the host lands on the line the
/// pointer is level with, as a browser's contain host does. When the
/// host has no inline flow at all, the host's edges decide instead:
///
/// - above host → first text descendant, offset 0;
/// - below host → last text descendant, offset = len;
/// - same vertical extent, past the right edge → last text end;
/// - same vertical extent, before the left edge → first text start.
pub(crate) fn clamp_to_contain_host(
    dom: &TuiDom,
    host: NodeId,
    mouse: MouseEvent,
) -> Option<Position> {
    use crate::runtime::hit_test::{nearest_inline_target_in_subtree, resolve_in_target};
    if let Some(target) = nearest_inline_target_in_subtree(dom, host, mouse.row)
        && let Some(pos) = resolve_in_target(dom, target, mouse.column, mouse.row)
    {
        return Some(pos);
    }
    let rect = dom.node(host).layout_rect()?;
    let below = (mouse.row as i32) >= rect.y + rect.height as i32;
    let above = (mouse.row as i32) < rect.y;
    let want_end = if below {
        true
    } else if above {
        false
    } else {
        (mouse.column as i32) >= rect.x + rect.width as i32
    };

    if want_end {
        let node = last_text_descendant(dom, host)?;
        Some(Position::new(node, text_len(dom, node)))
    } else {
        let node = first_text_descendant(dom, host)?;
        Some(Position::new(node, 0))
    }
}
