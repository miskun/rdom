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
