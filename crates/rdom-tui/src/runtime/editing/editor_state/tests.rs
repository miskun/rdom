//! B.4 unit tests — coalescing logic with injected clock.

use std::time::{Duration, Instant};

use rdom_core::{NodeId, Position};

use crate::TuiDom;
use crate::runtime::editing::editor_state::{COALESCE_WINDOW, EditEntry, EditKind, EditorState};

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

fn delete_entry(node: NodeId, start: usize, end: usize, old: &str) -> EditEntry {
    EditEntry {
        node,
        range: start..end,
        old: old.to_string(),
        new: String::new(),
        caret_before: Position::new(node, end),
        caret_after: Position::new(node, start),
        kind: EditKind::Delete,
    }
}

fn replace_entry(node: NodeId, start: usize, end: usize, old: &str, new: &str) -> EditEntry {
    EditEntry {
        node,
        range: start..end,
        old: old.to_string(),
        new: new.to_string(),
        caret_before: Position::new(node, end),
        caret_after: Position::new(node, start + new.len()),
        kind: EditKind::Replace,
    }
}

// ── Coalescing ────────────────────────────────────────────────────

#[test]
fn adjacent_inserts_within_window_coalesce() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    s.record(insert_entry(t, 1, "i"), t0 + Duration::from_millis(100));
    assert_eq!(s.undo_depth(), 1);
    // Coalesced entry extends `new` and advances `caret_after`.
    let e = s.pop_undo().unwrap();
    assert_eq!(e.new, "hi");
    assert_eq!(e.caret_after, Position::new(t, 2));
}

#[test]
fn inserts_past_window_do_not_coalesce() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    s.record(
        insert_entry(t, 1, "i"),
        t0 + COALESCE_WINDOW + Duration::from_millis(1),
    );
    assert_eq!(s.undo_depth(), 2, "past-window insert opens a new entry");
}

#[test]
fn non_adjacent_inserts_do_not_coalesce() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    // Caret moved (implicitly — the second insert starts at 5, not
    // at the previous entry's caret_after of 1).
    s.record(insert_entry(t, 5, "!"), t0 + Duration::from_millis(100));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn inserts_on_different_nodes_do_not_coalesce() {
    let (_dom, a, b) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(a, 0, "a"), t0);
    s.record(insert_entry(b, 0, "b"), t0 + Duration::from_millis(100));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn delete_never_coalesces_with_preceding_insert() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    s.record(delete_entry(t, 0, 1, "h"), t0 + Duration::from_millis(100));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn delete_never_coalesces_with_another_delete() {
    // Phase B.4 scope: only Insert coalesces. Delete-delete
    // coalescing is a polish item.
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(delete_entry(t, 4, 5, "o"), t0);
    s.record(delete_entry(t, 3, 4, "l"), t0 + Duration::from_millis(100));
    assert_eq!(s.undo_depth(), 2);
}

#[test]
fn replace_never_coalesces() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(replace_entry(t, 0, 2, "hi", "yo"), t0);
    s.record(insert_entry(t, 2, "!"), t0 + Duration::from_millis(100));
    assert_eq!(
        s.undo_depth(),
        2,
        "Replace is atomic — next insert opens a new entry"
    );
}

// ── Redo stack clearing ───────────────────────────────────────────

#[test]
fn recording_a_new_edit_clears_redo_stack() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    // Simulate Ctrl-Z: pop undo, push redo.
    let entry = s.pop_undo().unwrap();
    s.push_redo(entry);
    assert_eq!(s.redo_depth(), 1);

    // Fresh edit clears redo.
    s.record(insert_entry(t, 0, "x"), t0 + Duration::from_millis(100));
    assert_eq!(s.redo_depth(), 0);
}

// ── Post-pop state ────────────────────────────────────────────────

#[test]
fn pop_undo_breaks_coalescing_window() {
    let (_dom, t, _other) = make_two_text_nodes();
    let mut s = EditorState::new();
    let t0 = Instant::now();
    s.record(insert_entry(t, 0, "h"), t0);
    let _ = s.pop_undo();
    // A subsequent record within the old window must NOT try to
    // coalesce with a non-existent predecessor.
    s.record(insert_entry(t, 0, "i"), t0 + Duration::from_millis(50));
    assert_eq!(s.undo_depth(), 1);
    assert_eq!(s.pop_undo().unwrap().new, "i");
}
