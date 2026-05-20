//! Undo / redo — reverses + re-applies the last `EditEntry` from
//! the focused editable's `EditorState`.
//!
//! Called from `App::handle_event` as a default action on
//! `Ctrl-Z` / `Cmd-Z` (undo) and `Ctrl-Y` / `Cmd-Shift-Z` (redo).
//!
//! Semantics:
//!
//! - Undo replaces the post-edit bytes with the pre-edit `old`
//!   string, restores `caret_before`, and moves the entry to the
//!   redo stack.
//! - Redo replaces the pre-edit bytes with `new`, restores
//!   `caret_after`, and moves the entry back onto the undo stack.
//! - Both fire `input` (not `beforeinput`) on the editable. Browsers
//!   treat history operations as atomic — the mutation is already
//!   decided, handlers just get notified.
//! - No `beforeinput` means no cancellation. An app that wants
//!   veto power over undo/redo should listen for `keydown` and
//!   call `prevent_default` on the triggering `Ctrl-Z`.

use rdom_core::Selection;

use crate::node::nearest_editable_ancestor;
use crate::runtime::editing::editor_state::EditEntry;
use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Result of an undo/redo attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoOutcome {
    /// The history moved one step (a mutation was applied and
    /// `input` fired).
    Applied,
    /// No focused editable or the stack was empty — caller
    /// treats as a no-op.
    Noop,
}

/// Pop the top undo entry from the focused editable's state,
/// apply its reverse to the DOM, push onto the redo stack, and
/// fire `input`. Returns `Applied` when something happened.
pub fn undo(dom: &mut TuiDom) -> UndoOutcome {
    let Some(editable) = focused_editable(dom) else {
        return UndoOutcome::Noop;
    };
    let Some(entry) = pop_entry(dom, editable, StackSide::Undo) else {
        return UndoOutcome::Noop;
    };
    apply_reverse(dom, &entry);
    push_entry(dom, editable, entry.clone(), StackSide::Redo);
    // Fire `input` with HistoryUndo inputType — DOM convention is
    // `data: null` for history events; listeners read the new
    // value off the target's text content / `value` attribute.
    let _ = entry.old; // drop — the new state is reflected on the DOM, not in detail
    let mut ev = TuiEvent::input(rdom_core::InputType::HistoryUndo, None);
    let _ = dom.dispatch_tui_event(editable, &mut ev);
    UndoOutcome::Applied
}

/// Pop the top redo entry from the focused editable's state,
/// re-apply its forward edit, push back onto the undo stack, and
/// fire `input`. Returns `Applied` when something happened.
pub fn redo(dom: &mut TuiDom) -> UndoOutcome {
    let Some(editable) = focused_editable(dom) else {
        return UndoOutcome::Noop;
    };
    let Some(entry) = pop_entry(dom, editable, StackSide::Redo) else {
        return UndoOutcome::Noop;
    };
    apply_forward(dom, &entry);
    push_entry(dom, editable, entry.clone(), StackSide::Undo);
    let _ = entry.new; // drop — see undo() above
    let mut ev = TuiEvent::input(rdom_core::InputType::HistoryRedo, None);
    let _ = dom.dispatch_tui_event(editable, &mut ev);
    UndoOutcome::Applied
}

// ── Internals ──────────────────────────────────────────────────────

fn focused_editable(dom: &TuiDom) -> Option<rdom_core::NodeId> {
    let focused = dom.focused()?;
    nearest_editable_ancestor(dom, focused)
}

/// Re-apply `entry`'s forward edit to the DOM and restore
/// `caret_after`. Used by redo and by paste-integration in B.5.
fn apply_forward(dom: &mut TuiDom, entry: &EditEntry) {
    // Pre-apply state: the text node has `old` at
    // [range.start..range.start + old.len()). Replace with `new`.
    let end = entry.range.start + entry.old.len();
    let _ = dom
        .node_mut(entry.node)
        .edit_text(entry.range.start, end, &entry.new);
    dom.set_selection(Some(Selection::caret(entry.caret_after)));
}

/// Reverse `entry` — replace `new` with `old`, restore
/// `caret_before`. Used by undo.
fn apply_reverse(dom: &mut TuiDom, entry: &EditEntry) {
    let end = entry.range.start + entry.new.len();
    let _ = dom
        .node_mut(entry.node)
        .edit_text(entry.range.start, end, &entry.old);
    dom.set_selection(Some(Selection::caret(entry.caret_before)));
}

#[derive(Debug, Clone, Copy)]
enum StackSide {
    Undo,
    Redo,
}

fn pop_entry(dom: &mut TuiDom, editable: rdom_core::NodeId, side: StackSide) -> Option<EditEntry> {
    let mut node = dom.node_mut(editable);
    let state = node.ext_mut()?.editor_state.as_mut()?;
    match side {
        StackSide::Undo => state.pop_undo(),
        StackSide::Redo => state.pop_redo(),
    }
}

fn push_entry(dom: &mut TuiDom, editable: rdom_core::NodeId, entry: EditEntry, side: StackSide) {
    if let Some(ext) = dom.node_mut(editable).ext_mut()
        && let Some(state) = ext.editor_state.as_mut()
    {
        match side {
            StackSide::Undo => state.push_undo(entry),
            StackSide::Redo => state.push_redo(entry),
        }
    }
}

#[cfg(test)]
mod tests;
