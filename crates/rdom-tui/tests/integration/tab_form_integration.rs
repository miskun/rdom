//! End-to-end integration test mirroring the `tab_form` example.
//!
//! Drives an `App` with `TestBackend` via `handle_event` for the
//! full sequence:
//!
//! 1. Tab → focus first input
//! 2. Type characters → values appear in input
//! 3. Tab → focus second input
//! 4. Type characters → values appear
//! 5. Tab → focus submit button
//! 6. Enter → submit event fires, status text updates
//!
//! Each step's invariants are asserted right after the event,
//! including: the input's `value` attribute and text content reflect
//! typed characters; the submit handler reads the right values; the
//! status text gets the new content WITHOUT needing a subsequent
//! event to trigger redraw (Bug 2 regression test).
//!
//! This is the test that should have existed before the OOTB round.
//! It would have caught Bugs 1, 2, 3 together.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::ListenerOptions;
use rdom_tui::layout::{Direction, Size};
use rdom_tui::prelude::*;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::runtime::builtins::form;

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn ch(c: char) -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

#[test]
fn tab_form_full_loop_with_typing_and_submit() {
    // Build a form mirroring `crates/rdom-tui/examples/tab_form.rs`:
    // two `<input>` fields wrapped in a `<form>`, plus a `<button>`
    // submit. The submit handler writes the joined values into a
    // status text node.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let form_el = dom.create_element("form");

    // Two inputs with `name` attributes.
    let inputs: Vec<NodeId> = ["name", "email"]
        .iter()
        .map(|name| {
            let row = dom.create_element("row");
            let input = dom.create_element("input");
            dom.set_attribute(input, "type", "text").unwrap();
            dom.set_attribute(input, "name", name).unwrap();
            dom.append_child(row, input).unwrap();
            dom.append_child(form_el, row).unwrap();
            input
        })
        .collect();

    // Submit button.
    let submit_row = dom.create_element("row");
    let submit_btn = dom.create_element("button");
    let submit_text = dom.create_text_node("Submit");
    dom.append_child(submit_btn, submit_text).unwrap();
    dom.append_child(submit_row, submit_btn).unwrap();
    dom.append_child(form_el, submit_row).unwrap();

    let status = dom.create_element("status");
    let status_text = dom.create_text_node("(not submitted)");
    dom.append_child(status, status_text).unwrap();

    dom.append_child(root, form_el).unwrap();
    dom.append_child(root, status).unwrap();

    // Track submit fires so we can assert the handler ran exactly once.
    let submit_count = Rc::new(Cell::new(0u32));
    let submit_count_clone = submit_count.clone();

    dom.add_event_listener(form_el, "submit", ListenerOptions::default(), move |ctx| {
        submit_count_clone.set(submit_count_clone.get() + 1);
        let values = form::collect(ctx.dom, form_el);
        let mut msg = String::from("submitted: ");
        for (i, (name, value)) in values.iter().enumerate() {
            if i > 0 {
                msg.push_str(", ");
            }
            msg.push_str(&format!("{name}={value:?}"));
        }
        let _ = ctx.dom.node_mut(status_text).set_node_value(&msg);
    })
    .unwrap();

    // Minimal layout so the rows stack and have non-zero height.
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "form",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .gap(0),
        )
        .rule_unchecked(
            "row",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("input", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("status", TuiStyle::new().height(Size::Fixed(1)));

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // ── Step 1: Tab → focus the first input.
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(inputs[0]),
        "Tab should focus first input"
    );

    // After Tab to an editable input, the focus_node helper seeds a
    // collapsed caret at offset 0 of the input's first text child.
    // Without this (Bug 1), the next keystroke would silently no-op.
    assert!(
        app.dom().selection().is_some(),
        "focus_node should seed a caret for editable focus targets"
    );

    // ── Step 2: type "Alice" into the first input.
    for c in "Alice".chars() {
        app.handle_event(ch(c));
    }
    assert_eq!(
        app.dom().node(inputs[0]).get_attribute("value"),
        Some("Alice"),
        "typed characters must land in the first input's value attribute"
    );

    // ── Step 3: Tab → focus the second input.
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(inputs[1]),
        "Tab should advance to second input"
    );

    // ── Step 4: type "bob@x" into the second input.
    for c in "bob@x".chars() {
        app.handle_event(ch(c));
    }
    assert_eq!(
        app.dom().node(inputs[1]).get_attribute("value"),
        Some("bob@x"),
        "typed characters must land in the second input's value attribute"
    );
    // First input retains its earlier value.
    assert_eq!(
        app.dom().node(inputs[0]).get_attribute("value"),
        Some("Alice")
    );

    // ── Step 5: Tab → focus the submit button.
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(submit_btn),
        "Tab should advance to the submit button"
    );

    // Sanity: submit has NOT fired yet.
    assert_eq!(submit_count.get(), 0, "submit must not fire before Enter");
    let status_before = app
        .dom()
        .node(status_text)
        .node_value()
        .unwrap_or_default()
        .to_string();
    assert_eq!(status_before, "(not submitted)");

    // ── Step 6: Enter → fire submit, status text updates immediately.
    app.handle_event(key(KeyCode::Enter));

    // Submit fired exactly once with the typed values.
    assert_eq!(
        submit_count.get(),
        1,
        "Enter on submit must fire submit once"
    );
    let status_after = app
        .dom()
        .node(status_text)
        .node_value()
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        status_after, r#"submitted: name="Alice", email="bob@x""#,
        "status text must reflect typed values after Enter"
    );
}
