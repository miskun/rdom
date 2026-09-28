//! Undo-grouping unit tests. The selection serials are passed by hand:
//! `(n, n + 1)` is an edit that moved the caret itself; a `before` that
//! differs from the previous `after` is a foreign selection change.

use rdom_core::{NodeId, Position, SelectionSerial};

use crate::TuiDom;
use crate::runtime::editing::editor_state::{EditEntry, EditKind, EditorState};

/// A hand-made selection serial.
fn sel(n: u64) -> SelectionSerial {
    SelectionSerial::new(n)
}

fn make_two_text_nodes() -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let a = dom.create_text_node("");
    let b = dom.create_text_node("");
    (dom, a, b)
}

fn insert_entry(node: NodeId, at: usize, ch: &str) -> EditEntry {
    EditEntry {
        node,
        range: at..at,
        old: String::new(),
        new: ch.to_string(),
        caret_before: Position::new(node, at),
        caret_after: Position::new(node, at + ch.len()),
        kind: EditKind::Insert,
    }
}

fn backspace_entry(node: NodeId, start: usize, end: usize, old: &str) -> EditEntry {
    EditEntry {
        node,
        range: start..end,
        old: old.to_string(),
        new: String::new(),
        caret_before: Position::new(node, end),
        caret_after: Position::new(node, start),
        kind: EditKind::DeleteBackward,
    }
}

fn delete_forward_entry(node: NodeId, start: usize, end: usize, old: &str) -> EditEntry {
    EditEntry {
        kind: EditKind::DeleteForward,
        caret_before: Position::new(node, start),
        ..backspace_entry(node, start, end, old)
    }
}

fn standalone_entry(node: NodeId, start: usize, end: usize, old: &str, new: &str) -> EditEntry {
    EditEntry {
        node,
        range: start..end,
        old: old.to_string(),
        new: new.to_string(),
        caret_before: Position::new(node, end),
        caret_after: Position::new(node, start + new.len()),
        kind: EditKind::Standalone,
    }
}

#[test]
fn edit_kind_follows_the_input_type() {
    use rdom_core::InputType;
    assert_eq!(EditKind::of(&InputType::InsertText), EditKind::Insert);
    assert_eq!(EditKind::of(&InputType::InsertLineBreak), EditKind::Insert);
    assert_eq!(
        EditKind::of(&InputType::DeleteContentBackward),
        EditKind::DeleteBackward
    );
    assert_eq!(
        EditKind::of(&InputType::DeleteContentForward),
        EditKind::DeleteForward
    );
    for standalone in [
        InputType::InsertFromPaste,
        InputType::InsertFromDrop,
        InputType::InsertReplacementText,
        InputType::DeleteByCut,
        InputType::DeleteWordBackward,
    ] {
        assert_eq!(EditKind::of(&standalone), EditKind::Standalone);
    }
}

// ── Grouping ──────────────────────────────────────────────────────

#[test]
fn adjacent_inserts_join_the_open_group() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "h"), sel(0), sel(1));
    s.record(insert_entry(t, 1, "i"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 1);
    let step = s.pop_undo().unwrap();
    assert_eq!(step.len(), 1, "grouped into one part");
    let e = &step[0];
    assert_eq!(e.new, "hi");
    assert_eq!(e.caret_after, Position::new(t, 2));
}

#[test]
fn a_foreign_selection_change_closes_the_group() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "h"), sel(0), sel(1));
    // Serial 1 → 3: the caret moved away and back between the edits.
    s.record(insert_entry(t, 1, "i"), sel(3), sel(4));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn non_adjacent_inserts_do_not_join() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "h"), sel(0), sel(1));
    s.record(insert_entry(t, 5, "!"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn inserts_on_different_nodes_do_not_join() {
    let (_dom, a, b) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(a, 0, "a"), sel(0), sel(1));
    s.record(insert_entry(b, 0, "b"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn a_delete_does_not_join_a_typing_group() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "h"), sel(0), sel(1));
    s.record(backspace_entry(t, 0, 1, "h"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn a_backspace_run_folds_into_one_step() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    // "hello|" → Backspace ×2 → "hel|".
    s.record(backspace_entry(t, 4, 5, "o"), sel(0), sel(1));
    s.record(backspace_entry(t, 3, 4, "l"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 1);
    let e = &s.pop_undo().unwrap()[0];
    assert_eq!(e.range, 3..5);
    assert_eq!(e.old, "lo");
    assert_eq!(e.caret_before, Position::new(t, 5));
    assert_eq!(e.caret_after, Position::new(t, 3));
}

#[test]
fn a_forward_delete_run_folds_into_one_step_apart_from_backspace() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    // "he|llo" → Delete ×2 → "he|o".
    s.record(delete_forward_entry(t, 2, 3, "l"), sel(0), sel(1));
    s.record(delete_forward_entry(t, 2, 3, "l"), sel(1), sel(2));
    assert_eq!(s.undo_depth(), 1);
    s.record(backspace_entry(t, 1, 2, "e"), sel(2), sel(3));
    assert_eq!(s.undo_depth(), 2, "Backspace after Delete opens a step");
    let _ = s.pop_undo();
    let e = &s.pop_undo().unwrap()[0];
    assert_eq!(e.range, 2..4);
    assert_eq!(e.old, "ll");
}

#[test]
fn a_standalone_edit_neither_joins_nor_is_joined() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "a"), sel(0), sel(1));
    s.record(standalone_entry(t, 1, 1, "", "XY"), sel(1), sel(2));
    s.record(insert_entry(t, 3, "b"), sel(2), sel(3));
    assert_eq!(s.undo_depth(), 3);
}

// ── Redo stack clearing ───────────────────────────────────────────

#[test]
fn recording_a_new_edit_clears_redo_stack() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "h"), sel(0), sel(1));
    // Simulate Ctrl-Z: pop undo, push redo.
    let entry = s.pop_undo().unwrap();
    s.push_redo(entry);
    assert_eq!(s.redo_depth(), 1);

    // Fresh edit clears redo.
    s.record(insert_entry(t, 0, "x"), sel(1), sel(2));
    assert_eq!(s.redo_depth(), 0);
}

// ── Post-pop state ────────────────────────────────────────────────

#[test]
fn pop_undo_closes_the_group() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    s.record(insert_entry(t, 0, "a"), sel(0), sel(1));
    s.record(backspace_entry(t, 0, 1, "a"), sel(1), sel(2));
    let _ = s.pop_undo();
    // The serial is unchanged, but the group under the popped step must
    // not absorb the next edit.
    s.record(insert_entry(t, 1, "b"), sel(2), sel(3));
    assert_eq!(s.undo_depth(), 2);
    assert_eq!(s.pop_undo().unwrap()[0].new, "b");
}
