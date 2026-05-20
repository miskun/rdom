//! End-to-end rendered-buffer integration test for `<input>`.
//!
//! `tab_form_integration.rs` already covers the DOM-level path
//! (Tab focus, typing, `value` attribute, submit). This test sits
//! at the architect-required bar: drive `App` with real key events,
//! then re-render and assert that what got typed actually appears
//! in the painted buffer.
//!
//! Catches the same class of bug that the textarea wrap regression
//! exposed — feature works at the DOM level but doesn't surface in
//! paint.

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::NodeType;
use rdom_tui::layout::Size;
use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect, Terminal, TestBackend};
use rdom_tui::runtime::app::App;

/// Dispatch an event AND run the deferred redraw — the way the
/// production run loop does. Bare `handle_event` mutates the DOM
/// but doesn't re-cascade or re-layout.
fn dispatch<B: rdom_tui::render::Backend>(app: &mut App<B>, event: CtEvent) {
    app.handle_event(event);
    let _ = app.draw_if_dirty();
}

fn shift(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

mod common;
use common::render;

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn ch(c: char) -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn input_text(dom: &TuiDom, id: rdom_tui::NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(v) = child.node_value()
        {
            out.push_str(v);
        }
    }
    out
}

/// Concatenate visible symbols in `buf` between `(x_start, y)` and
/// `(x_end, y)`, skipping spacer cells.
fn row_slice(buf: &Buffer, y: u16, x_start: u16, x_end: u16) -> String {
    let mut out = String::new();
    for x in x_start..x_end {
        if let Some(c) = buf.cell(x, y)
            && !c.is_spacer()
        {
            out.push_str(c.symbol());
        }
    }
    out
}

#[test]
fn typing_into_input_renders_to_buffer() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "text").unwrap();
    dom.set_attribute(input, "name", "name").unwrap();
    dom.append_child(root, input).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // ── Step 1: Tab focuses the input.
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(input),
        "Tab should focus the input"
    );

    // ── Step 2: type "hello".
    for c in "hello".chars() {
        app.handle_event(ch(c));
    }

    // DOM-level: value attribute and text content reflect typed chars.
    assert_eq!(
        app.dom().node(input).get_attribute("value"),
        Some("hello"),
        "value attribute mirrors typed chars"
    );
    let value = input_text(app.dom(), input);
    assert_eq!(value, "hello");

    // Rendered-buffer assertion: render the DOM at the same width as
    // the App's viewport and verify "hello" appears on the input's
    // row. The input lives at y=0 (root has no padding, input is the
    // first child).
    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 5));
    let row0 = row_slice(&buf, 0, 0, 40);
    assert!(
        row0.contains("hello"),
        "row 0 should contain typed text 'hello'; got {row0:?}"
    );
}

// ── D2: input parity mirror tests ──────────────────────────────
//
// These mirror the user-visible behaviors that `textarea_integration.rs`
// pins for `<textarea>` — pinning `<input>`'s share of editing
// parity (caret, arrow keys, Home/End, Shift+arrows).

fn build_app_with_input(initial: &str) -> (App<TestBackend>, rdom_tui::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "text").unwrap();
    if !initial.is_empty() {
        dom.set_attribute(input, "value", initial).unwrap();
    }
    dom.append_child(root, input).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    let _ = app.draw_if_dirty();
    (app, input)
}

#[test]
fn focused_input_paints_visible_caret() {
    // Tab focuses the input → caret appears as a painted cell with
    // inverted fg/bg of the surrounding text (the default
    // caret-color: auto behavior).
    let (mut app, _input) = build_app_with_input("");
    dispatch(&mut app, key(KeyCode::Tab));

    // After focus, a caret cell should exist. The simplest check
    // is: at least one cell on row 0 differs in bg from a non-
    // caret field cell. We don't pin exact colors — those vary
    // by terminal and palette — but the caret must produce SOME
    // visible difference vs an unfocused cell.
    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 5));
    let mut distinct_bgs = std::collections::HashSet::new();
    for x in 0..20 {
        if let Some(cell) = buf.cell(x, 0) {
            distinct_bgs.insert(format!("{:?}", cell.bg));
        }
    }
    assert!(
        distinct_bgs.len() >= 2,
        "focused input must paint a caret cell whose bg differs from the field; got bgs {distinct_bgs:?}"
    );
}

#[test]
fn input_arrow_keys_move_caret_horizontally() {
    // Type "abc", then Left twice — caret should be at offset 1.
    // The visible signature is which cell is the caret cell (the
    // bg-inverted one). Position 1 means cell 2 in the painted
    // row (1-cell padding + offset 1).
    let (mut app, input) = build_app_with_input("");
    dispatch(&mut app, key(KeyCode::Tab));
    for c in "abc".chars() {
        dispatch(&mut app, ch(c));
    }
    dispatch(&mut app, key(KeyCode::Left));
    dispatch(&mut app, key(KeyCode::Left));

    // Caret position lives on the DOM selection.
    let sel = app.dom().selection().expect("selection set after typing");
    assert!(
        sel.is_collapsed(),
        "caret movement leaves a collapsed selection"
    );
    let text_id = app
        .dom()
        .node(input)
        .child_nodes()
        .find(|n| n.node_type() == NodeType::Text)
        .map(|n| n.id())
        .unwrap();
    assert_eq!(sel.focus.node, text_id);
    assert_eq!(
        sel.focus.offset, 1,
        "two Lefts from end of 'abc' → offset 1"
    );
}

#[test]
fn input_home_end_jump_to_line_edges() {
    let (mut app, input) = build_app_with_input("");
    dispatch(&mut app, key(KeyCode::Tab));
    for c in "abcde".chars() {
        dispatch(&mut app, ch(c));
    }
    // Caret is at offset 5; Home → 0, End → 5.
    let text_id = app
        .dom()
        .node(input)
        .child_nodes()
        .find(|n| n.node_type() == NodeType::Text)
        .map(|n| n.id())
        .unwrap();

    dispatch(&mut app, key(KeyCode::Home));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.focus, rdom_core::Position::new(text_id, 0));

    dispatch(&mut app, key(KeyCode::End));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.focus, rdom_core::Position::new(text_id, 5));
}

#[test]
fn input_shift_arrows_extend_selection() {
    // Type "abcdef", caret at offset 6. Shift+Left twice should
    // extend selection back to offset 4 (anchor=6, focus=4).
    let (mut app, input) = build_app_with_input("");
    dispatch(&mut app, key(KeyCode::Tab));
    for c in "abcdef".chars() {
        dispatch(&mut app, ch(c));
    }

    dispatch(&mut app, shift(KeyCode::Left));
    dispatch(&mut app, shift(KeyCode::Left));

    let text_id = app
        .dom()
        .node(input)
        .child_nodes()
        .find(|n| n.node_type() == NodeType::Text)
        .map(|n| n.id())
        .unwrap();
    let sel = app.dom().selection().unwrap();
    assert!(!sel.is_collapsed());
    assert_eq!(sel.anchor, rdom_core::Position::new(text_id, 6));
    assert_eq!(sel.focus, rdom_core::Position::new(text_id, 4));
}
