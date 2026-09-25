//! `user-select` used value (CSS UI 4 §6.1) — a pure function of the
//! computed style and the tree, shared by paint (the selection
//! highlight skips `none` content), hit-testing, and the runtime's
//! selection behaviors (drag, keyboard extension, clipboard), which
//! live in `runtime::selection::user_select`.
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
//! walks that already hold the parent's used value. [`all_host`] /
//! [`contain_host`] find the element whose used value governs a
//! selection starting at a node.

use rdom_core::{NodeId, NodeType};

use crate::TuiDom;
use crate::layout::UserSelect;
use crate::node::TuiNodeExt;

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

#[cfg(test)]
mod tests;
