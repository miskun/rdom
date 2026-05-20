//! `perform_edit` — the editor-aware wrapper around
//! `rdom_core::Dom::edit_text`.
//!
//! Wires the full `beforeinput` → mutation → `input` flow:
//!
//! 1. Resolve the target editable element from the selection.
//! 2. Fire `beforeinput` on the editable (cancelable).
//! 3. If not prevented, apply the byte-range edit.
//! 4. Update `Dom::selection` to a caret at the post-edit position.
//! 5. Fire `input` on the editable (non-cancelable).
//! 6. B.4 will add an undo/redo entry push here; B.1/B.2 stub.
//!
//! Used by:
//! - Character insertion in `App::handle_event` (keydown → printable
//!   char → `perform_edit`).
//! - Backspace / Delete in B.3.
//! - Paste default in B.5.

use std::time::Instant;

use rdom_core::{NodeId, Position, Selection};

use crate::node::nearest_editable_ancestor;
use crate::runtime::editing::editor_state::{EditEntry, EditKind, EditorState};
use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// A proposed byte-range edit on a single text node.
///
/// - `node`: the text node being edited.
/// - `range`: the byte range to replace (may be empty for a pure
///   insert).
/// - `text`: the replacement text (may be empty for a pure delete).
#[derive(Debug, Clone)]
pub struct Edit {
    pub node: NodeId,
    pub range: std::ops::Range<usize>,
    pub text: String,
}

/// Result of a `perform_edit` call. `Applied` = the edit committed,
/// caret is at `caret_after`. `Prevented` = `beforeinput` was
/// cancelled by a handler; the DOM is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcome {
    Applied,
    Prevented,
    /// The edit's target isn't editable or didn't resolve to an
    /// editable ancestor. Callers should treat this as "no-op."
    NoEditableTarget,
}

/// Apply an edit on a text node, firing the full `beforeinput` →
/// mutate → `input` sequence. Returns `Applied` / `Prevented` /
/// `NoEditableTarget` for callers to discriminate.
///
/// Pre-edit selection state: whatever the caller set. Post-edit
/// caret: at `caret_after(edit)` — byte offset where the
/// replacement text ends. The edit is recorded on the editable's
/// `EditorState` for undo/redo (Phase B.4).
pub fn perform_edit(dom: &mut TuiDom, edit: Edit) -> EditOutcome {
    let editable = match nearest_editable_ancestor(dom, edit.node) {
        Some(id) => id,
        None => return EditOutcome::NoEditableTarget,
    };

    // `readonly` blocks edits but keeps the element editable for
    // focus / selection routing (HTML-faithful contrast with
    // `disabled`, which makes `is_editable` return false outright).
    // Callers see the same `Prevented` outcome they'd get from a
    // user-cancelled `beforeinput` — no event fires.
    if dom.node(editable).has_attribute("readonly") {
        return EditOutcome::Prevented;
    }

    // Snapshot caret-before + the bytes we're about to replace.
    // Both are needed to build the undo entry after a successful
    // edit; grabbing them pre-mutation avoids re-reading the DOM.
    let caret_before = dom
        .selection()
        .map(|s| s.focus)
        .unwrap_or_else(|| Position::new(edit.node, edit.range.start));
    let old_text = match dom.node(edit.node).node_value() {
        Some(s) => {
            let start = edit.range.start.min(s.len());
            let end = edit.range.end.min(s.len()).max(start);
            s[start..end].to_string()
        }
        None => return EditOutcome::NoEditableTarget,
    };

    // Classify the edit and build the detail payload once — both
    // beforeinput and input share the same shape.
    let input_type = classify_input_type(&edit);
    let data = if edit.text.is_empty() {
        None
    } else {
        Some(edit.text.clone())
    };

    // Fire `beforeinput` — cancelable.
    let mut before = TuiEvent::before_input(input_type.clone(), data.clone());
    let _ = dom.dispatch_tui_event(editable, &mut before);
    if before.event.default_prevented() {
        return EditOutcome::Prevented;
    }

    // Apply the edit. `edit_text` validates byte boundaries and
    // fires `Mutation::CharacterDataChanged`. Offset errors bail
    // out as Prevented — caller treats both as "nothing happened."
    if dom
        .node_mut(edit.node)
        .edit_text(edit.range.start, edit.range.end, &edit.text)
        .is_err()
    {
        return EditOutcome::Prevented;
    }

    // Update the selection to a caret at the edit's end-of-insert.
    let caret_offset = edit.range.start + edit.text.len();
    let caret_after = Position::new(edit.node, caret_offset);
    dom.set_selection(Some(Selection::caret(caret_after)));

    // Record on the editable's history stack. Lazily allocate
    // `EditorState` on first edit (keeps non-editable elements at
    // 8 bytes / no heap). `record` handles coalescing internally.
    if let Some(ext) = dom.node_mut(editable).ext_mut() {
        let entry = EditEntry {
            node: edit.node,
            range: edit.range.clone(),
            old: old_text,
            new: edit.text.clone(),
            caret_before,
            caret_after,
            kind: classify_kind(&edit),
        };
        ext.editor_state
            .get_or_insert_with(|| Box::new(EditorState::new()))
            .record(entry, Instant::now());
    }

    // `<input>` value-attribute mirror — keep the attribute in
    // lockstep with the live text content, so apps reading
    // `get_attribute("value")` see the post-edit string. No-op for
    // `<textarea>` and `contenteditable` (their value is just the
    // text content). Runs BEFORE the `input` event so listeners
    // can read the up-to-date attribute.
    crate::runtime::builtins::input::mirror_to_attribute(dom, editable);

    // Fire `input` — non-cancelable post-commit signal. Same
    // detail shape as the `beforeinput` we fired above.
    let mut after = TuiEvent::input(input_type, data);
    let _ = dom.dispatch_tui_event(editable, &mut after);

    EditOutcome::Applied
}

/// Map an `Edit` to the DOM `InputEvent.inputType` value that
/// best describes it. Pure inserts → `InsertText`; selection
/// replacements → `InsertReplacementText`; pure deletions →
/// `DeleteContentBackward` (we don't track caret direction
/// here, and backspace is the common case).
fn classify_input_type(edit: &Edit) -> rdom_core::InputType {
    let has_text = !edit.text.is_empty();
    let has_range = edit.range.start != edit.range.end;
    match (has_text, has_range) {
        (true, false) => rdom_core::InputType::InsertText,
        (true, true) => rdom_core::InputType::InsertReplacementText,
        (false, true) => rdom_core::InputType::DeleteContentBackward,
        // Empty range + empty text — perform_edit upstream
        // shouldn't call us with this, but be explicit.
        (false, false) => rdom_core::InputType::InsertText,
    }
}

/// Classify an `Edit` for coalescing purposes.
///
/// - Empty range + non-empty text → `Insert` (coalescable).
/// - Non-empty range + empty text → `Delete`.
/// - Anything else (range + text, or both empty) → `Replace`,
///   which is non-coalescable and gets its own undo step.
fn classify_kind(edit: &Edit) -> EditKind {
    let range_empty = edit.range.start == edit.range.end;
    match (range_empty, edit.text.is_empty()) {
        (true, false) => EditKind::Insert,
        (false, true) => EditKind::Delete,
        _ => EditKind::Replace,
    }
}

/// Convenience: insert `text` at the current selection. If the
/// selection is a non-collapsed range, the range is replaced;
/// otherwise the text is inserted at the caret. No-op when no
/// selection exists or it spans multiple text nodes (the MVP
/// single-text-node restriction per V2 §4.2 decision #2).
///
/// Returns `Applied` / `Prevented` / `NoEditableTarget` — same
/// contract as `perform_edit`.
pub fn insert_at_selection(dom: &mut TuiDom, text: &str) -> EditOutcome {
    let Some(sel) = dom.selection() else {
        return EditOutcome::NoEditableTarget;
    };
    let range = selection_to_single_node_range(sel).unwrap_or(None);
    let Some((node, start, end)) = range else {
        return EditOutcome::NoEditableTarget;
    };
    perform_edit(
        dom,
        Edit {
            node,
            range: start..end,
            text: text.to_string(),
        },
    )
}

/// Narrow `Selection` to a single-text-node `(node, start, end)`
/// triple. Returns `Ok(None)` when the selection spans different
/// nodes (MVP restriction). The `Ok(Some(_))` / `Ok(None)` shape
/// lets callers compose with `?` cleanly.
fn selection_to_single_node_range(sel: &Selection) -> Result<Option<(NodeId, usize, usize)>, ()> {
    if sel.anchor.node != sel.focus.node {
        return Ok(None);
    }
    let (start, end) = if sel.anchor.offset <= sel.focus.offset {
        (sel.anchor.offset, sel.focus.offset)
    } else {
        (sel.focus.offset, sel.anchor.offset)
    };
    Ok(Some((sel.anchor.node, start, end)))
}

/// Helper exposed for callers that want the caret position
/// resulting from an edit without actually performing it. Currently
/// used by the app-level character-insert handler to sanity-check
/// before routing through `perform_edit`.
pub fn caret_after(edit: &Edit) -> Position {
    Position::new(edit.node, edit.range.start + edit.text.len())
}

/// Utility for B.5 (paste) — build an edit that replaces the
/// current single-node selection range with `text`. Returns `None`
/// when the selection doesn't narrow to a single-node range.
pub fn edit_for_selection(dom: &TuiDom, text: String) -> Option<Edit> {
    let sel = dom.selection()?;
    let (node, start, end) = selection_to_single_node_range(sel).ok()??;
    Some(Edit {
        node,
        range: start..end,
        text,
    })
}

#[cfg(test)]
mod tests;
