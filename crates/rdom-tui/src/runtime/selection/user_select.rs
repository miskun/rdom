//! `user-select` policy — `UserSelect::{None, All, Contain}` apply
//! logic shared by mouse drag, keyboard extension, and clipboard
//! serialization.
//!
//! The CSS property + cascade rung lives in `rdom-style`; this
//! module owns the runtime *behavior* the property controls.
//!
//! ## Behaviors
//!
//! - **`None`** — the subtree is unselectable. Drag, keyboard
//!   extension, and clipboard serialize all skip it.
//! - **`All`** — the host is selected atomically. A click anywhere
//!   inside expands the selection to span the host's full text;
//!   subsequent drag-extend and keyboard-extend are suppressed
//!   while the focus remains inside the host.
//! - **`Contain`** — selections started inside the host cannot
//!   escape. Drag-extend whose hit position falls outside the host
//!   clamps to the in-host line nearest the pointer's row.
//!
//! The host of `All` / `Contain` is the outermost element in the run
//! of ancestors computing that value ([`host_with`]): the property
//! inherits, so the nearest match would be the innermost descendant.
//!
//! `Auto` / `Text` are no-ops at the policy layer — they reflect
//! the default "selectable" state the drag pipeline assumes.

use crossterm::event::MouseEvent;

use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::UserSelect;
use crate::node::{TuiNodeExt, first_text_descendant, last_text_descendant, text_len};

/// First ancestor of `id` (inclusive) whose computed `user-select`
/// matches `value`. Returns the host element so callers can scope
/// per-value behavior to its subtree.
pub(crate) fn ancestor_with(dom: &TuiDom, id: NodeId, value: UserSelect) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if let Some(c) = dom.node(n).computed()
            && c.user_select == value
        {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// The host of a `user-select` value for `id`: the **outermost**
/// element of the contiguous run of ancestors (inclusive) whose
/// computed `user-select` is `value`. `user-select` inherits, so the
/// nearest match is usually the innermost descendant of the element
/// that declared it — a paragraph inside a `contain` host, not the
/// host — and clamping or spanning that element instead of the host
/// is the multi-paragraph bug (`EDIT-2`). Climbing the run finds the
/// declaring element without needing the specified value.
pub(crate) fn host_with(dom: &TuiDom, id: NodeId, value: UserSelect) -> Option<NodeId> {
    let mut host = ancestor_with(dom, id, value)?;
    while let Some(parent) = dom.node(host).parent_node().map(|p| p.id()) {
        match dom.node(parent).computed() {
            Some(c) if c.user_select == value => host = parent,
            _ => break,
        }
    }
    Some(host)
}

/// True iff `id` or any ancestor has `user-select: none` in its
/// computed style. Skip-list for the selection algorithm — used
/// by mouse hit-test, keyboard extension, and clipboard serialize.
pub(crate) fn has_none_ancestor(dom: &TuiDom, id: NodeId) -> bool {
    ancestor_with(dom, id, UserSelect::None).is_some()
}

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
