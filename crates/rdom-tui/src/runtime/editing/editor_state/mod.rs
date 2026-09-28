//! Per-editable mutable state — undo/redo history + undo grouping.
//!
//! ## Undo grouping (`P7-UNDO-COALESCE-1`)
//!
//! Blink's `TypingCommand` model (Chrome is the de-facto reference for
//! text-field undo; the HTML and Input Events specs leave grouping to
//! the UA): a typing run, a Backspace run and a forward-Delete run are
//! each one undo step; a switch between them, a selection change the
//! edit did not make (arrow keys, clicks, script, focus moving), paste,
//! cut, undo and redo close the group. No timer and no word-boundary
//! split. The rules live in [`EditKind`] and [`EditorState::record`].
//!
//! Lives on the editable element's `TuiExt.editor_state` so
//! `Drop` of the node takes the state with it (no side table, no
//! manual GC). Lazily populated on first edit to keep the field
//! at 8 bytes for non-editable elements.

use std::ops::Range;

use rdom_core::{InputType, NodeId, Position, SelectionSerial};

/// The kind of edit an entry describes — which undo group it can join.
///
/// Derived from the edit's Input Events `inputType`
/// ([`EditKind::of`]). The grouping model is Blink's `TypingCommand`
/// (`third_party/blink/renderer/core/editing/commands/typing_command.cc`),
/// the de-facto reference for `<input>` / `<textarea>` /
/// `contenteditable`:
///
/// - a run of typed text (`insertText`, `insertLineBreak`,
///   `insertParagraph`) is one step, whitespace and word boundaries
///   included;
/// - a run of Backspaces is one step, and a run of forward Deletes is
///   another (Blink: "group continuous delete commands alone");
/// - switching between those kinds starts a new step;
/// - everything else — paste, cut, drop, replacement, word deletion —
///   is a step of its own and closes the group.
///
/// Browsers use no timer: a pause does not split a run. What closes a
/// group besides a kind switch is a selection change the edit did not
/// make itself (see [`EditorState::record`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditKind {
    /// Typed text. Joins an open typing group.
    Insert,
    /// Backspace (`deleteContentBackward`). Joins an open Backspace group.
    DeleteBackward,
    /// Forward Delete (`deleteContentForward`). Joins an open Delete group.
    DeleteForward,
    /// Paste, cut, drop, replacement text, word deletion, … — always its
    /// own step, and closes whatever group was open.
    Standalone,
}

impl EditKind {
    /// The undo-grouping kind of an edit with `input_type`.
    pub fn of(input_type: &InputType) -> Self {
        match input_type {
            InputType::InsertText | InputType::InsertLineBreak | InputType::InsertParagraph => {
                Self::Insert
            }
            InputType::DeleteContentBackward => Self::DeleteBackward,
            InputType::DeleteContentForward => Self::DeleteForward,
            _ => Self::Standalone,
        }
    }
}

/// One step in the undo/redo history.
///
/// The byte `range` describes what the *original* text looked like
/// — `old` is the text that was at `[range.start..range.end)`;
/// after the edit, `new` occupies `[range.start..range.start+new.len())`.
///
/// - **To undo**: replace `[range.start..range.start+new.len())`
///   with `old`, then move caret to `caret_before`.
/// - **To redo**: replace `[range.start..range.start+old.len())`
///   with `new`, then move caret to `caret_after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditEntry {
    pub node: NodeId,
    pub range: Range<usize>,
    pub old: String,
    pub new: String,
    pub caret_before: Position,
    pub caret_after: Position,
    pub kind: EditKind,
}

/// One step of history: the per-node edits a single user action made,
/// in application order. A plain edit is one part; a cross-node edit
/// (a selection spanning text nodes replaced in one go) is several,
/// undone together in reverse order and redone together in order
/// (`EDIT-1`).
pub type HistoryItem = Vec<EditEntry>;

/// Undo + redo stacks for a single editable element.
///
/// Every committed edit pushes onto `undo`; an `undo()` call pops
/// from `undo` and pushes onto `redo`. A fresh (non-undo) edit
/// clears `redo` — browser-standard behavior where branching off
/// the history discards the abandoned future.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EditorState {
    undo: Vec<HistoryItem>,
    redo: Vec<HistoryItem>,
    /// The open group: `Some(serial)` while the top undo step can still
    /// absorb the next edit, `serial` being the
    /// [`Dom::selection_serial`](rdom_core::Dom::selection_serial) the
    /// last recorded edit left behind. `None` once the group is closed
    /// (a standalone or compound step, undo, redo).
    open_group: Option<SelectionSerial>,
    /// Sticky cell-column for vertical caret motion.
    ///
    /// `Some(x)` when the previous applied caret action was Up or
    /// Down. The next vertical motion uses this `x` as the target
    /// column (clamped to the target line's width) rather than the
    /// current caret column — so moving Down from a long line to a
    /// shorter one and back lands at the original column, not the
    /// clamped one. This is the canonical browser behavior.
    ///
    /// `None` after any action that isn't a vertical caret move:
    /// typing, deletion, horizontal arrow keys, Home, End, mouse
    /// click. Cleared via [`clear_sticky_x`](Self::clear_sticky_x).
    sticky_x: Option<u16>,
}

impl EditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the sticky column for vertical caret motion. See the
    /// field's doc-comment for semantics.
    pub fn sticky_x(&self) -> Option<u16> {
        self.sticky_x
    }

    /// Set the sticky column for vertical caret motion. Idempotent —
    /// calling repeatedly with the same value is a no-op. Called by
    /// the vertical-motion path the first time a vertical arrow is
    /// pressed (initialized from the caret's current cell column).
    pub fn set_sticky_x(&mut self, x: u16) {
        self.sticky_x = Some(x);
    }

    /// Drop the sticky column. Called from every code path other
    /// than vertical caret motion — typing, horizontal arrows, etc.
    pub fn clear_sticky_x(&mut self) {
        self.sticky_x = None;
    }

    /// Record a just-applied edit. Either joins the open group on top
    /// of the undo stack or pushes a fresh step. Clears the redo stack —
    /// branching off history throws away the abandoned future.
    ///
    /// `selection_before` is the document's
    /// [`selection_serial`](rdom_core::Dom::selection_serial) read before
    /// the edit moved the caret, `selection_after` the reading after.
    /// The edit joins the group only when `selection_before` is the
    /// serial the group's last edit left — any selection change in
    /// between (an arrow key, a click, script, focus moving away and
    /// back) closes it, even one that returns the caret to the same
    /// spot. Then the kinds must match and the edit must continue the
    /// run on the same text node (see [`EditKind`]).
    pub fn record(
        &mut self,
        entry: EditEntry,
        selection_before: SelectionSerial,
        selection_after: SelectionSerial,
    ) {
        self.redo.clear();

        let open = self.open_group == Some(selection_before);
        let joins = open
            && match self.undo.last() {
                Some(top) if top.len() == 1 => Self::continues(&top[0], &entry),
                _ => false,
            };
        let kind = entry.kind;
        if joins {
            let top = &mut self.undo.last_mut().expect("joins implies a top step")[0];
            Self::extend_in_place(top, &entry);
        } else {
            self.undo.push(vec![entry]);
        }
        self.open_group = match kind {
            EditKind::Standalone => None,
            _ => Some(selection_after),
        };
    }

    /// Record a multi-node edit as one history step. Never joins a
    /// group, and closes the open one. Clears redo.
    pub fn record_compound(&mut self, parts: HistoryItem) {
        if parts.is_empty() {
            return;
        }
        self.redo.clear();
        self.undo.push(parts);
        self.open_group = None;
    }

    /// Pop the top undo entry. Caller reverses it against the DOM
    /// and pushes the (unchanged) entry onto the redo stack via
    /// `push_redo`. Returns `None` when the undo stack is empty.
    pub fn pop_undo(&mut self) -> Option<HistoryItem> {
        let entry = self.undo.pop()?;
        // Undo closes the group: the next edit starts a new step.
        self.open_group = None;
        Some(entry)
    }

    /// Pop the top redo entry. Caller re-applies it against the DOM
    /// and pushes back onto the undo stack via `push_undo`.
    pub fn pop_redo(&mut self) -> Option<HistoryItem> {
        let entry = self.redo.pop()?;
        self.open_group = None;
        Some(entry)
    }

    /// Called by undo machinery to move an entry from undo → redo
    /// after reversing its DOM effect.
    pub fn push_redo(&mut self, entry: HistoryItem) {
        self.redo.push(entry);
    }

    /// Called by redo machinery to move an entry from redo → undo
    /// after re-applying its DOM effect.
    pub fn push_undo(&mut self, entry: HistoryItem) {
        self.undo.push(entry);
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    // ── Coalescing internals ────────────────────────────────────────

    /// Whether `next` continues the run `top` describes: same kind (not
    /// standalone), same text node, and adjacent in the way the kind
    /// implies — typing inserts at the caret the run left, Backspace
    /// deletes just before the run's start, Delete removes the bytes
    /// that slid into the run's start.
    fn continues(top: &EditEntry, next: &EditEntry) -> bool {
        if top.kind != next.kind || top.node != next.node {
            return false;
        }
        match next.kind {
            EditKind::Insert => next.range.is_empty() && next.range.start == top.caret_after.offset,
            EditKind::DeleteBackward => {
                top.new.is_empty() && next.new.is_empty() && next.range.end == top.range.start
            }
            EditKind::DeleteForward => {
                top.new.is_empty() && next.new.is_empty() && next.range.start == top.range.start
            }
            EditKind::Standalone => false,
        }
    }

    /// Fold `next` into `top` — called only after
    /// [`continues`](Self::continues) returned true — so one undo
    /// reverses both and one redo re-applies both.
    fn extend_in_place(top: &mut EditEntry, next: &EditEntry) {
        match next.kind {
            // `range` / `old` stay those of the run's first edit (a
            // typing run may have begun by replacing a selection).
            EditKind::Insert => top.new.push_str(&next.new),
            EditKind::DeleteBackward => {
                top.old.insert_str(0, &next.old);
                top.range.start = next.range.start;
            }
            EditKind::DeleteForward => {
                top.old.push_str(&next.old);
                top.range.end += next.old.len();
            }
            EditKind::Standalone => unreachable!("standalone edits never join a group"),
        }
        top.caret_after = next.caret_after;
    }
}

#[cfg(test)]
mod tests;
