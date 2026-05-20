//! Keyboard-driven selection — `Shift+arrow`, `Shift+Ctrl+arrow`,
//! `Ctrl-A`.
//!
//! Called as a default action from `App::handle_event` after the
//! `keydown` dispatch — handlers can suppress these by calling
//! `event.prevent_default()` on the key event.
//!
//! ## Scope
//!
//! - **`Ctrl-A` / `Cmd-A`**: select every character under the
//!   focused element (or the document root if nothing focused).
//! - **`Shift+Left` / `Shift+Right`**: extend the selection's focus
//!   by one grapheme in the focus's text node.
//! - **`Shift+Ctrl+Left` / `Shift+Ctrl+Right`**: extend by one
//!   word (via `unicode-segmentation` word boundaries).
//!
//! Cross-text-node traversal (extending past the end of one text
//! node into the next) is node-local only — degraded but safe. The
//! full document-order walk is a follow-up once a richer caret
//! model lands.
//!
//! ## Not handled here
//!
//! - **Bare `Left` / `Right`** (without Shift): these are caret
//!   movement for editable elements (`<input>` / `<textarea>` /
//!   `contenteditable`). A non-editable view has nothing to move.
//! - **`Shift+Up` / `Shift+Down`**: line-based extension needs
//!   reverse cell→line mapping on the IFC, which the caret
//!   infrastructure will bring.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use rdom_core::{NodeId, NodeType, Position, Selection};

use crate::TuiDom;

/// Try to consume `key` as a selection-default action. Returns
/// `true` when a selection change happened — caller marks a
/// redraw. Never fires events; selection updates go through
/// `Dom::set_selection` which notifies observers.
pub(crate) fn try_handle_key(dom: &mut TuiDom, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') if ctrl => select_all_under_focus(dom),
        KeyCode::Left if shift && ctrl => extend_by_word(dom, Dir::Backward),
        KeyCode::Right if shift && ctrl => extend_by_word(dom, Dir::Forward),
        KeyCode::Left if shift => extend_by_grapheme(dom, Dir::Backward),
        KeyCode::Right if shift => extend_by_grapheme(dom, Dir::Forward),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Forward,
    Backward,
}

/// `Ctrl-A` / `Cmd-A`: select all text content under the focused
/// element, or under the document root when nothing is focused.
/// Returns `false` when there's no text to select (leaves any
/// existing selection untouched).
fn select_all_under_focus(dom: &mut TuiDom) -> bool {
    let root = dom.focused().unwrap_or_else(|| dom.root());
    let Some(first) = first_text_node(dom, root) else {
        return false;
    };
    let last = last_text_node(dom, root).unwrap_or(first);
    let end_offset = text_len(dom, last);

    let next = Selection::new(Position::new(first, 0), Position::new(last, end_offset));
    if dom.selection() == Some(&next) {
        return false;
    }
    dom.set_selection(Some(next));
    true
}

/// Extend the selection's focus by one grapheme in the given
/// direction. No-op when there's no selection, when the focus
/// isn't in a text node, or when we're at the text node's
/// boundary in the requested direction (v1 doesn't cross nodes).
fn extend_by_grapheme(dom: &mut TuiDom, dir: Dir) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let text = match text_of(dom, sel.focus.node) {
        Some(t) => t,
        None => return false,
    };
    let offset = sel.focus.offset.min(text.len());
    let new_offset = match dir {
        Dir::Forward => next_grapheme_byte(&text, offset),
        Dir::Backward => prev_grapheme_byte(&text, offset),
    };
    let Some(new_offset) = new_offset else {
        return false;
    };
    if new_offset == sel.focus.offset {
        return false;
    }
    dom.set_selection(Some(Selection::new(
        sel.anchor,
        Position::new(sel.focus.node, new_offset),
    )));
    true
}

/// Extend the selection's focus by one word in the given
/// direction. Word boundaries come from `UnicodeSegmentation`'s
/// `split_word_bound_indices` (Unicode TR29). Node-local for v1.
fn extend_by_word(dom: &mut TuiDom, dir: Dir) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let text = match text_of(dom, sel.focus.node) {
        Some(t) => t,
        None => return false,
    };
    let offset = sel.focus.offset.min(text.len());
    let new_offset = match dir {
        Dir::Forward => next_word_byte(&text, offset),
        Dir::Backward => prev_word_byte(&text, offset),
    };
    let Some(new_offset) = new_offset else {
        return false;
    };
    if new_offset == sel.focus.offset {
        return false;
    }
    dom.set_selection(Some(Selection::new(
        sel.anchor,
        Position::new(sel.focus.node, new_offset),
    )));
    true
}

// ── grapheme / word helpers ─────────────────────────────────────────

/// Next grapheme boundary after `offset`, or `None` at end.
pub(crate) fn next_grapheme_byte(text: &str, offset: usize) -> Option<usize> {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .find(|&i| i > offset)
}

/// Previous grapheme boundary before `offset`, or `None` at start.
pub(crate) fn prev_grapheme_byte(text: &str, offset: usize) -> Option<usize> {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .rev()
        .find(|&i| i < offset)
}

/// Next word boundary strictly after `offset`, or `None` at end.
/// Boundaries come from TR29's word segmentation, so punctuation
/// and CJK character runs land on sensible stops.
pub(crate) fn next_word_byte(text: &str, offset: usize) -> Option<usize> {
    text.split_word_bound_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .find(|&i| i > offset)
}

/// Previous word boundary strictly before `offset`, or `None` at
/// start.
pub(crate) fn prev_word_byte(text: &str, offset: usize) -> Option<usize> {
    text.split_word_bound_indices()
        .map(|(i, _)| i)
        .rfind(|&i| i < offset)
}

// ── DOM traversal helpers ───────────────────────────────────────────

/// Read the text content of a text node. `None` when `id` isn't a
/// text node.
fn text_of(dom: &TuiDom, id: NodeId) -> Option<String> {
    dom.node(id).node_value().map(|s| s.to_string())
}

/// Byte length of a text node's data. `0` when not a text node.
fn text_len(dom: &TuiDom, id: NodeId) -> usize {
    dom.node(id).node_value().map(|s| s.len()).unwrap_or(0)
}

/// First text-node descendant (DFS, document order) of `root`,
/// inclusive of `root` itself.
fn first_text_node(dom: &TuiDom, root: NodeId) -> Option<NodeId> {
    if is_text_node(dom, root) {
        return Some(root);
    }
    for child in dom.node(root).child_nodes() {
        if let Some(found) = first_text_node(dom, child.id()) {
            return Some(found);
        }
    }
    None
}

/// Last text-node descendant (DFS, reverse document order) of
/// `root`, inclusive of `root` itself.
fn last_text_node(dom: &TuiDom, root: NodeId) -> Option<NodeId> {
    if is_text_node(dom, root) {
        return Some(root);
    }
    let children: Vec<NodeId> = dom.node(root).child_nodes().map(|c| c.id()).collect();
    for child in children.into_iter().rev() {
        if let Some(found) = last_text_node(dom, child) {
            return Some(found);
        }
    }
    None
}

fn is_text_node(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).node_type() == NodeType::Text
}

#[cfg(test)]
mod tests;
