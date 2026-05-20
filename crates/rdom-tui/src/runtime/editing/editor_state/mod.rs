//! Per-editable mutable state — undo/redo history + coalescing
//! metadata.
//!
//! Lives on the editable element's `TuiExt.editor_state` so
//! `Drop` of the node takes the state with it (no side table, no
//! manual GC). Lazily populated on first edit to keep the field
//! at 8 bytes for non-editable elements.

use std::ops::Range;
use std::time::{Duration, Instant};

use rdom_core::{NodeId, Position};

/// Max gap between successive `Insert` edits that still coalesce
/// into a single undo entry. Matches the "typical editor" 500 ms
/// pause window — if the user stops typing for half a second, the
/// next keystroke starts a new undo chunk.
pub const COALESCE_WINDOW: Duration = Duration::from_millis(500);

/// The kind of edit an entry describes. Drives coalescing: only
/// `Insert` coalesces with an adjacent prior `Insert`; everything
/// else is always a fresh entry.
///
/// `Replace` covers "typing with a range selection" — the single
/// `edit_text` call both deletes the selected range and inserts
/// the typed char. Browsers treat such an edit as atomic for undo
/// (Ctrl-Z brings the whole range back), so it doesn't coalesce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    /// Inserted text into an empty range (pure insert). Coalesces
    /// with the prior entry if same-node + adjacent + within
    /// `COALESCE_WINDOW`.
    Insert,
    /// Deleted a non-empty range, replacement empty (Backspace,
    /// Delete, range-selection Delete).
    Delete,
    /// Non-empty range replaced with non-empty text (typing over
    /// selection, paste-over-selection).
    Replace,
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

/// Undo + redo stacks for a single editable element.
///
/// Every committed edit pushes onto `undo`; an `undo()` call pops
/// from `undo` and pushes onto `redo`. A fresh (non-undo) edit
/// clears `redo` — browser-standard behavior where branching off
/// the history discards the abandoned future.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EditorState {
    undo: Vec<EditEntry>,
    redo: Vec<EditEntry>,
    /// Time the last entry's edit committed. Used together with
    /// `COALESCE_WINDOW` to decide whether the next insert extends
    /// the pending entry or starts a fresh one.
    last_commit: Option<Instant>,
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

    /// Record a just-applied edit. Either coalesces into the top
    /// of the undo stack (when the rules permit) or pushes a fresh
    /// entry. Clears the redo stack — branching off history throws
    /// away the abandoned future.
    ///
    /// `now` is injected so tests can drive coalescing without
    /// racing the real clock; production callers pass
    /// `Instant::now()`.
    pub fn record(&mut self, entry: EditEntry, now: Instant) {
        self.redo.clear();

        if Self::can_coalesce_with_previous(&self.undo, &entry, self.last_commit, now) {
            let top = self.undo.last_mut().unwrap();
            Self::extend_in_place(top, &entry);
        } else {
            self.undo.push(entry);
        }
        self.last_commit = Some(now);
    }

    /// Pop the top undo entry. Caller reverses it against the DOM
    /// and pushes the (unchanged) entry onto the redo stack via
    /// `push_redo`. Returns `None` when the undo stack is empty.
    pub fn pop_undo(&mut self) -> Option<EditEntry> {
        let entry = self.undo.pop()?;
        // Taking from undo breaks any pending coalescing window —
        // a subsequent edit must start a new entry regardless of
        // timing.
        self.last_commit = None;
        Some(entry)
    }

    /// Pop the top redo entry. Caller re-applies it against the DOM
    /// and pushes back onto the undo stack via `push_undo`.
    pub fn pop_redo(&mut self) -> Option<EditEntry> {
        let entry = self.redo.pop()?;
        self.last_commit = None;
        Some(entry)
    }

    /// Called by undo machinery to move an entry from undo → redo
    /// after reversing its DOM effect.
    pub fn push_redo(&mut self, entry: EditEntry) {
        self.redo.push(entry);
    }

    /// Called by redo machinery to move an entry from redo → undo
    /// after re-applying its DOM effect.
    pub fn push_undo(&mut self, entry: EditEntry) {
        self.undo.push(entry);
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    // ── Coalescing internals ────────────────────────────────────────

    /// Decide whether `new_entry` should extend the top-of-undo
    /// entry rather than push a fresh one. The rules:
    ///
    /// - Both entries must be `Insert` kind.
    /// - Same text node.
    /// - New entry's insert position is right at the previous
    ///   entry's `caret_after` (adjacent typing).
    /// - Less than `COALESCE_WINDOW` elapsed since the previous
    ///   commit.
    fn can_coalesce_with_previous(
        undo: &[EditEntry],
        new_entry: &EditEntry,
        last_commit: Option<Instant>,
        now: Instant,
    ) -> bool {
        if new_entry.kind != EditKind::Insert {
            return false;
        }
        let Some(top) = undo.last() else { return false };
        if top.kind != EditKind::Insert {
            return false;
        }
        if top.node != new_entry.node {
            return false;
        }
        if new_entry.range.start != top.caret_after.offset {
            return false;
        }
        let Some(last) = last_commit else {
            return false;
        };
        if now.duration_since(last) > COALESCE_WINDOW {
            return false;
        }
        true
    }

    /// Extend `top` with `incoming` — called only after
    /// `can_coalesce_with_previous` returns true. Merges the two
    /// inserts into one undo step.
    fn extend_in_place(top: &mut EditEntry, incoming: &EditEntry) {
        // Both entries are pure inserts on the same node; the new
        // entry's range is collapsed (start == end) at
        // `top.caret_after`.
        top.new.push_str(&incoming.new);
        top.caret_after = incoming.caret_after;
        // `old` stays the empty string (pure insert). `range`
        // stays the original (the position where insertion began).
    }
}

#[cfg(test)]
mod tests;
