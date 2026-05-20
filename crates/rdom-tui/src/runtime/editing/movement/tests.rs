//! B.3 tests — caret movement + deletion.
//!
//! Covers the full Phase B.3 key list: Backspace, Delete, bare
//! Left/Right (grapheme + collapse-selection), Ctrl+Left/Right
//! (word), Home/End (line), Ctrl+Home/End (doc), Up/Down (line).
//! Plus end-to-end via `App::handle_event` for the most-used keys.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{LayoutExt, Rect, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::editing::movement::try_handle_movement_key;
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

fn editable_paragraph(text: &str) -> (TuiDom, NodeId, NodeId) {
    editable_paragraph_with_width(text, 40)
}

fn editable_paragraph_with_width(text: &str, width: u16) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(width)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, width + 20, 10));
    (dom, p, t)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

// ── Backspace ───────────────────────────────────────────────────────

#[test]
fn backspace_deletes_grapheme_before_caret() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let consumed =
        try_handle_movement_key(&mut dom, key(KeyCode::Backspace, KeyModifiers::empty()));
    assert!(consumed);

    assert_eq!(dom.node(t).node_value(), Some("hell"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 4));
}

#[test]
fn backspace_on_cjk_removes_whole_grapheme() {
    let (mut dom, _p, t) = editable_paragraph("中文");
    dom.set_selection(Some(Selection::caret(Position::new(t, 6))));

    try_handle_movement_key(&mut dom, key(KeyCode::Backspace, KeyModifiers::empty()));

    assert_eq!(dom.node(t).node_value(), Some("中"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3));
}

#[test]
fn backspace_with_selection_deletes_range() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    try_handle_movement_key(&mut dom, key(KeyCode::Backspace, KeyModifiers::empty()));

    assert_eq!(dom.node(t).node_value(), Some("hello "));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 6));
}

#[test]
fn backspace_at_start_is_noop_but_consumed() {
    let (mut dom, _p, t) = editable_paragraph("hi");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    let consumed =
        try_handle_movement_key(&mut dom, key(KeyCode::Backspace, KeyModifiers::empty()));
    assert!(consumed);
    assert_eq!(dom.node(t).node_value(), Some("hi"));
}

// ── Delete ──────────────────────────────────────────────────────────

#[test]
fn delete_removes_grapheme_after_caret() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    try_handle_movement_key(&mut dom, key(KeyCode::Delete, KeyModifiers::empty()));

    assert_eq!(dom.node(t).node_value(), Some("helo"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

#[test]
fn delete_with_selection_deletes_range() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 6),
    )));

    try_handle_movement_key(&mut dom, key(KeyCode::Delete, KeyModifiers::empty()));

    assert_eq!(dom.node(t).node_value(), Some("world"));
}

#[test]
fn delete_at_end_is_noop_but_consumed() {
    let (mut dom, _p, t) = editable_paragraph("hi");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_movement_key(&mut dom, key(KeyCode::Delete, KeyModifiers::empty()));
    assert!(consumed);
    assert_eq!(dom.node(t).node_value(), Some("hi"));
}

// ── Bare arrows ─────────────────────────────────────────────────────

#[test]
fn left_moves_caret_one_grapheme() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

#[test]
fn right_moves_caret_one_grapheme() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    try_handle_movement_key(&mut dom, key(KeyCode::Right, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3));
}

#[test]
fn left_on_cjk_moves_past_full_grapheme() {
    let (mut dom, _p, t) = editable_paragraph("中文");
    dom.set_selection(Some(Selection::caret(Position::new(t, 6))));

    try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3));
}

#[test]
fn left_with_range_selection_collapses_to_start() {
    // Browser behavior: bare Left with a range collapses to the
    // range's start (not "move by one from focus").
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 2),
        Position::new(t, 7),
    )));

    try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::empty()));

    let sel = dom.selection().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.focus, Position::new(t, 2));
}

#[test]
fn right_with_range_selection_collapses_to_end() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 2),
        Position::new(t, 7),
    )));

    try_handle_movement_key(&mut dom, key(KeyCode::Right, KeyModifiers::empty()));

    let sel = dom.selection().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.focus, Position::new(t, 7));
}

#[test]
fn left_at_start_is_noop_but_consumed() {
    let (mut dom, _p, t) = editable_paragraph("hi");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    let consumed = try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::empty()));
    assert!(consumed);
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 0));
}

// ── Ctrl+arrows (word) ─────────────────────────────────────────────

#[test]
fn ctrl_left_moves_to_previous_word() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 11))));

    try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::CONTROL));
    // Previous word boundary — on "hello world" boundaries are at
    // 0, 5, 6, 11. From 11 → 6.
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 6));
}

#[test]
fn ctrl_right_moves_to_next_word() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    try_handle_movement_key(&mut dom, key(KeyCode::Right, KeyModifiers::CONTROL));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

// ── Home / End ─────────────────────────────────────────────────────

#[test]
fn home_moves_caret_to_line_start() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 8))));

    try_handle_movement_key(&mut dom, key(KeyCode::Home, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 0));
}

#[test]
fn end_moves_caret_to_line_end() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    try_handle_movement_key(&mut dom, key(KeyCode::End, KeyModifiers::empty()));
    // End-of-single-line = end-of-text for an unwrapped paragraph.
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 11));
}

#[test]
fn home_on_wrapped_second_line_goes_to_that_lines_start() {
    // Width 6 wraps "hello world" into "hello" / "world".
    let (mut dom, _p, t) = editable_paragraph_with_width("hello world", 6);
    dom.set_selection(Some(Selection::caret(Position::new(t, 9)))); // inside "world"

    try_handle_movement_key(&mut dom, key(KeyCode::Home, KeyModifiers::empty()));
    // Second line starts at byte 6 (the 'w').
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 6));
}

#[test]
fn ctrl_home_moves_caret_to_doc_start() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 4))));

    try_handle_movement_key(&mut dom, key(KeyCode::Home, KeyModifiers::CONTROL));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 0));
}

#[test]
fn ctrl_end_moves_caret_to_doc_end() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 1))));

    try_handle_movement_key(&mut dom, key(KeyCode::End, KeyModifiers::CONTROL));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

// ── Up / Down ──────────────────────────────────────────────────────

#[test]
fn down_moves_caret_to_next_line_in_wrapped_paragraph() {
    // Wrapped: line 0 = "hello" (cells 0-4), line 1 = "world"
    // (cells 0-4). From cell 2 of line 0 (byte 2 = 'l') → cell 2
    // of line 1 (byte 8 = 'r').
    let (mut dom, _p, t) = editable_paragraph_with_width("hello world", 6);
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    try_handle_movement_key(&mut dom, key(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 8));
}

#[test]
fn up_moves_caret_to_prev_line() {
    let (mut dom, _p, t) = editable_paragraph_with_width("hello world", 6);
    dom.set_selection(Some(Selection::caret(Position::new(t, 8)))); // line 1 cell 2

    try_handle_movement_key(&mut dom, key(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

#[test]
fn up_on_first_line_is_noop_but_consumed() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_movement_key(&mut dom, key(KeyCode::Up, KeyModifiers::empty()));
    assert!(consumed);
    // Still at byte 2.
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

// ── Shift+arrow is NOT handled here ────────────────────────────────

#[test]
fn shift_arrow_is_not_claimed_by_movement_handler() {
    // Shift+Left belongs to selection-keyboard (Phase 6.5.3). This
    // handler must return false so the outer dispatch continues.
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_movement_key(&mut dom, key(KeyCode::Left, KeyModifiers::SHIFT));
    assert!(!consumed);
}

// ── End-to-end via App::handle_event ────────────────────────────────

fn test_app(dom: TuiDom, sheet: Stylesheet, viewport: Rect) -> App<TestBackend> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

#[test]
fn typing_then_backspace_roundtrips() {
    // Mimics real interactive use: type a char, then Backspace it.
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('!'), KeyModifiers::empty())));
    assert_eq!(app.dom().node(t).node_value(), Some("hello!"));

    app.handle_event(CtEvent::Key(key(KeyCode::Backspace, KeyModifiers::empty())));
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
}

#[test]
fn arrow_key_movement_through_app() {
    let (mut dom, p, t) = editable_paragraph("abc");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Left, KeyModifiers::empty())));
    assert_eq!(app.dom().selection().unwrap().focus, Position::new(t, 2));

    app.handle_event(CtEvent::Key(key(KeyCode::Home, KeyModifiers::empty())));
    assert_eq!(app.dom().selection().unwrap().focus, Position::new(t, 0));
}
