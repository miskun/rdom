//! Keyboard selection tests — exercise `try_handle_key` against
//! synthetic key events + a minimal `<p>` / `<span>` IFC fixture.
//!
//! Covered scenarios:
//! - `Ctrl-A` over a text-bearing focused element selects all of it.
//! - `Ctrl-A` with nothing focused selects the whole document.
//! - `Shift+Right` / `Shift+Left` extends the focus by one grapheme
//!   (ASCII + CJK — CJK's full grapheme, not a half-width step).
//! - `Shift+Ctrl+Right` / `Shift+Ctrl+Left` extends by word boundary.
//! - No-op cases: bare arrow, plain Ctrl, no selection in context,
//!   prevent_default upstream.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{LayoutExt, Rect};
use crate::runtime::selection::keyboard::try_handle_key;
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

fn prepare(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
}

/// A `<p>hello world<span/></p>` fixture so the IFC check (requires
/// an inline element child) passes. Returns (dom, p, text_node).
fn paragraph(text: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
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
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 60, 10));
    (dom, p, t)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

// ── Ctrl-A ──────────────────────────────────────────────────────────

#[test]
fn ctrl_a_with_focused_element_selects_its_text() {
    let (mut dom, p, t) = paragraph("hello world");
    dom.set_focused(Some(p));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('a'), KeyModifiers::CONTROL));

    assert!(consumed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, "hello world".len()));
}

#[test]
fn ctrl_a_without_focus_selects_whole_document() {
    let (mut dom, _p, t) = paragraph("abc");

    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('a'), KeyModifiers::CONTROL));

    assert!(consumed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 3));
}

#[test]
fn ctrl_a_uppercase_also_consumed() {
    // Some terminals emit 'A' with Ctrl+Shift+A — still Select-All.
    let (mut dom, _p, t) = paragraph("abc");
    let consumed = try_handle_key(
        &mut dom,
        key(
            KeyCode::Char('A'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ),
    );
    assert!(consumed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.focus, Position::new(t, 3));
}

#[test]
fn ctrl_a_is_idempotent() {
    let (mut dom, _p, _t) = paragraph("abc");
    assert!(try_handle_key(
        &mut dom,
        key(KeyCode::Char('a'), KeyModifiers::CONTROL)
    ));
    // Second Ctrl-A: selection is already everything → no state change.
    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert!(!consumed);
}

#[test]
fn ctrl_a_on_empty_document_returns_false() {
    let mut dom: TuiDom = TuiDom::new();
    prepare(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 10));
    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert!(!consumed);
    assert!(dom.selection().is_none());
}

#[test]
fn cmd_a_super_modifier_also_consumed() {
    // macOS terminals may report Cmd-A with the SUPER modifier.
    let (mut dom, _p, t) = paragraph("abc");
    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('a'), KeyModifiers::SUPER));
    assert!(consumed);
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3));
}

// ── Shift+Right / Shift+Left (grapheme) ─────────────────────────────

#[test]
fn shift_right_extends_focus_forward_one_grapheme() {
    let (mut dom, _p, t) = paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));

    assert!(consumed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 2));
    assert_eq!(sel.focus, Position::new(t, 3));
}

#[test]
fn shift_left_extends_focus_backward_one_grapheme() {
    let (mut dom, _p, t) = paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Left, KeyModifiers::SHIFT));

    assert!(consumed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 3));
    assert_eq!(sel.focus, Position::new(t, 2));
}

#[test]
fn shift_right_on_cjk_skips_full_grapheme() {
    // "中" is 3 bytes; one Shift+Right at offset 0 should land at 3,
    // not at a sub-codepoint position.
    let (mut dom, _p, t) = paragraph("中文");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));

    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3));
}

#[test]
fn shift_right_at_end_of_text_is_noop() {
    let (mut dom, _p, t) = paragraph("ab");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));

    assert!(!consumed);
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

#[test]
fn shift_left_at_start_of_text_is_noop() {
    let (mut dom, _p, t) = paragraph("ab");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Left, KeyModifiers::SHIFT));

    assert!(!consumed);
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 0));
}

#[test]
fn shift_arrow_without_selection_is_noop() {
    let (mut dom, _p, _t) = paragraph("hello");
    assert!(dom.selection().is_none());

    let consumed = try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));

    assert!(!consumed);
    assert!(dom.selection().is_none());
}

#[test]
fn bare_arrow_without_shift_is_not_consumed() {
    // Bare Left / Right are caret-movement keys for editable
    // elements (Phase 14.7). The selection module leaves them alone.
    let (mut dom, _p, t) = paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::empty()));

    assert!(!consumed);
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 2));
}

#[test]
fn shift_right_extends_existing_range_not_collapse() {
    // Starting from a non-collapsed selection anchor=1 focus=4,
    // Shift+Right moves focus to 5 (still extending forward).
    let (mut dom, _p, t) = paragraph("hello");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 4),
    )));

    try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 1));
    assert_eq!(sel.focus, Position::new(t, 5));
}

// ── Shift+Ctrl+Right / Shift+Ctrl+Left (word) ──────────────────────

#[test]
fn shift_ctrl_right_extends_to_next_word_boundary() {
    // "hello world" — word boundaries at 0, 5, 6, 11.
    // From offset 2 in "hel|lo world", next boundary > 2 is 5.
    let (mut dom, _p, t) = paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    try_handle_key(
        &mut dom,
        key(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
    );

    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

#[test]
fn shift_ctrl_right_from_word_start_goes_to_next_boundary() {
    // At offset 0 ("|hello world"), next boundary is 5.
    let (mut dom, _p, t) = paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    try_handle_key(
        &mut dom,
        key(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
    );

    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

#[test]
fn shift_ctrl_left_goes_to_previous_word_boundary() {
    // "hello |world" at offset 6, prev boundary < 6 is 5 (space
    // boundary) → focus moves to 5.
    let (mut dom, _p, t) = paragraph("hello world");
    dom.set_selection(Some(Selection::caret(Position::new(t, 6))));

    try_handle_key(
        &mut dom,
        key(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
    );

    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

#[test]
fn shift_ctrl_right_at_end_is_noop() {
    let (mut dom, _p, t) = paragraph("hi");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let consumed = try_handle_key(
        &mut dom,
        key(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
    );

    assert!(!consumed);
}

#[test]
fn shift_ctrl_left_at_start_is_noop() {
    let (mut dom, _p, t) = paragraph("hi");
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    let consumed = try_handle_key(
        &mut dom,
        key(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
    );

    assert!(!consumed);
}

// ── Non-handled keys ────────────────────────────────────────────────

#[test]
fn plain_ctrl_b_is_not_consumed() {
    let (mut dom, _p, _t) = paragraph("abc");
    let consumed = try_handle_key(&mut dom, key(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert!(!consumed);
}

#[test]
fn shift_arrow_inside_user_select_all_does_not_shrink_selection() {
    // Per CSS UI: `user-select: all` selects the element atomically.
    // Any keyboard extension that would split the all-host's
    // selection must be suppressed — the host stays fully selected.
    // (Drag-extend has the same gate in `drag::extend`.)
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("token");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40))
                .user_select(crate::layout::UserSelect::All),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 60, 10));

    // Pre-state: full-host selection (anchor=0, focus=end).
    let full = Selection::new(Position::new(t, 0), Position::new(t, "token".len()));
    dom.set_selection(Some(full));

    // Shift+Left would normally retract focus by one grapheme.
    // Under `user-select: all` the call returns false and the
    // selection stays at its all-host extent.
    let consumed = try_handle_key(&mut dom, key(KeyCode::Left, KeyModifiers::SHIFT));
    assert!(
        !consumed,
        "Shift+Left inside user-select: all must not be consumed"
    );
    assert_eq!(
        dom.selection().copied(),
        Some(full),
        "selection must remain at the all-host's full extent"
    );

    // Same for Shift+Right.
    let consumed = try_handle_key(&mut dom, key(KeyCode::Right, KeyModifiers::SHIFT));
    assert!(!consumed);
    assert_eq!(dom.selection().copied(), Some(full));
}

#[test]
fn shift_up_down_extend_selection_with_line_clamps() {
    // Shift+Up at top-of-content extends selection focus to line
    // START (offset 0). Shift+Down at bottom extends to line END
    // (= text.len()). Matches the caret-move clamps that bare
    // Up / Down ship.
    let (mut dom, _p, t) = paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    // Shift+Up: focus to start of line 0 = offset 0.
    let consumed_up = try_handle_key(&mut dom, key(KeyCode::Up, KeyModifiers::SHIFT));
    assert!(
        consumed_up,
        "Shift+Up extends selection focus to line start"
    );
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 2));
    assert_eq!(sel.focus, Position::new(t, 0));

    // Reset to caret at offset 2 and try Shift+Down.
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));
    let consumed_down = try_handle_key(&mut dom, key(KeyCode::Down, KeyModifiers::SHIFT));
    assert!(consumed_down, "Shift+Down extends to line end");
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 2));
    assert_eq!(sel.focus, Position::new(t, 5)); // end of "hello"
}
