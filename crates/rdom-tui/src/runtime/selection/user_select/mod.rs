//! `user-select` policy — `UserSelect::{None, All, Contain}` apply
//! logic shared by mouse drag, keyboard extension, hit-test, paint
//! (the selection overlay) and clipboard serialization.
//!
//! The CSS property + cascade rung lives in `rdom-style`; this
//! module owns the property's **used value** and the runtime
//! *behavior* it controls.
//!
//! ## Used value (CSS UI 4 §6.1)
//!
//! `user-select` is not inherited: an element without a declaration
//! computes `auto`. The used value of `auto` is:
//!
//! - `none` on `::before` / `::after` — generated content has no DOM
//!   position here, so no selection can reach it (nothing to resolve);
//! - `contain` on an editable element ([`TuiNodeExt::is_editable`]:
//!   text controls and `contenteditable` editing hosts);
//! - otherwise `all` if the parent's used value is `all`, `none` if
//!   it is `none`, and `text` in every other case — so `contain` never
//!   propagates, and an explicit `text` stops a `none` / `all`.
//!
//! [`used_value`] resolves it for any node (a text node takes its
//! parent element's). [`resolve_child`] is the same rule top-down, for
//! walks that already hold the parent's used value.
//!
//! ## Behaviors
//!
//! - **`None`** — unselectable ([`is_unselectable`]). Drag, keyboard
//!   extension, the highlight and clipboard serialize all skip it.
//! - **`All`** — the host ([`all_host`]: the outermost ancestor whose
//!   used value is `all`) is selected atomically. A click anywhere
//!   inside expands the selection to span the host's full text;
//!   subsequent drag-extend and keyboard-extend are suppressed while
//!   the focus remains inside the host.
//! - **`Contain`** — selections started inside the host
//!   ([`contain_host`]: the nearest ancestor whose used value is
//!   `contain`, i.e. that declares it or is an editable `auto`
//!   element) cannot escape. Drag-extend whose hit position falls
//!   outside the host clamps to the in-host line nearest the
//!   pointer's row.
//!
//! `Text` is a no-op at the policy layer — the default "selectable"
//! state the drag pipeline assumes.

use crossterm::event::MouseEvent;

use rdom_core::{NodeId, NodeType, Position, Selection};

use crate::TuiDom;
use crate::layout::UserSelect;
use crate::node::{TuiNodeExt, first_text_descendant, last_text_descendant, text_len};

/// The used value an element takes from its own computed value alone,
/// or `None` when that is `auto` on a non-editable element (the parent
/// decides). Non-elements and uncascaded elements declare nothing.
fn own_used_value(dom: &TuiDom, id: NodeId) -> Option<UserSelect> {
    let node = dom.node(id);
    match node.computed()?.user_select {
        UserSelect::Auto if node.is_editable() => Some(UserSelect::Contain),
        UserSelect::Auto => None,
        v => Some(v),
    }
}

/// What an `auto`, non-editable element inherits from a parent whose
/// used value is `parent`: `all` and `none` carry, anything else is
/// `text`.
fn from_parent(parent: UserSelect) -> UserSelect {
    match parent {
        UserSelect::All | UserSelect::None => parent,
        _ => UserSelect::Text,
    }
}

/// The used value of element `id` given its parent's used value
/// (CSS UI 4 §6.1), for top-down walks. Never `Auto`.
pub(crate) fn resolve_child(dom: &TuiDom, id: NodeId, parent: UserSelect) -> UserSelect {
    own_used_value(dom, id).unwrap_or_else(|| from_parent(parent))
}

/// The used `user-select` of `id` (CSS UI 4 §6.1). A text node (or any
/// non-element) takes its parent element's. Never `Auto`: the root's
/// parent counts as `text`.
///
/// Walks up to the nearest element that decides its own value; every
/// element in between is `auto`, so only `all` / `none` carry down to
/// `id` — `contain` / `text` resolve its descendants to `text`.
pub(crate) fn used_value(dom: &TuiDom, id: NodeId) -> UserSelect {
    let mut is_self = true;
    let mut cur = Some(id);
    while let Some(n) = cur {
        if let Some(v) = own_used_value(dom, n) {
            return if is_self { v } else { from_parent(v) };
        }
        if dom.node(n).node_type() == NodeType::Element {
            is_self = false;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    UserSelect::Text
}

/// True iff the used `user-select` of `id` is `none` — the skip-list
/// for the selection algorithm (hit-test, drag, keyboard extension,
/// the highlight, clipboard serialize).
pub(crate) fn is_unselectable(dom: &TuiDom, id: NodeId) -> bool {
    used_value(dom, id) == UserSelect::None
}

/// The `user-select: all` host of `id`: the outermost ancestor
/// (inclusive) whose used value is `all`. CSS UI 4 §6.1: a selection
/// containing part of such an element contains all of it, so an `all`
/// element nested in another — or an explicit `text` inside one — is
/// selected with the outer host.
pub(crate) fn all_host(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut host = None;
    let mut cur = Some(id);
    while let Some(n) = cur {
        if own_used_value(dom, n) == Some(UserSelect::All) {
            host = Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    host
}

/// The `user-select: contain` host of `id`: the nearest ancestor
/// (inclusive) whose used value is `contain` — one that declares it,
/// or an editable `auto` element. `contain` does not propagate, so a
/// `contain` card inside a `contain` panel is its own host.
pub(crate) fn contain_host(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if own_used_value(dom, n) == Some(UserSelect::Contain) {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
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

#[cfg(test)]
mod tests;
