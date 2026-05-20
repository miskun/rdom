//! `<input type="number">` filter + stepping tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::input;
use crate::style::Stylesheet;

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn number_input_app(initial: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "number").unwrap();
    dom.set_attribute(inp, "value", initial).unwrap();
    dom.append_child(root, inp).unwrap();

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    let t = app
        .dom()
        .node(inp)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    (app, inp, t)
}

// ── Numeric filter ────────────────────────────────────────────────

#[test]
fn typing_a_digit_in_number_input_inserts() {
    let (mut app, inp, t) = number_input_app("12");
    app.dom_mut().set_focused(Some(inp));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));

    app.handle_event(key(KeyCode::Char('3')));

    assert_eq!(input::value(app.dom(), inp), "123");
}

#[test]
fn typing_a_letter_in_number_input_is_filtered_out() {
    let (mut app, inp, t) = number_input_app("12");
    app.dom_mut().set_focused(Some(inp));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));

    app.handle_event(key(KeyCode::Char('a')));

    assert_eq!(input::value(app.dom(), inp), "12");
}

#[test]
fn typing_decimal_point_is_allowed() {
    let (mut app, inp, t) = number_input_app("3");
    app.dom_mut().set_focused(Some(inp));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 1))));

    app.handle_event(key(KeyCode::Char('.')));
    app.handle_event(key(KeyCode::Char('1')));
    app.handle_event(key(KeyCode::Char('4')));

    assert_eq!(input::value(app.dom(), inp), "3.14");
}

#[test]
fn backspace_in_number_input_still_works() {
    // The filter only blocks non-numeric INSERTS (detail non-empty
    // + non-numeric chars). Pure deletes should pass through.
    let (mut app, inp, t) = number_input_app("123");
    app.dom_mut().set_focused(Some(inp));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 3))));

    app.handle_event(key(KeyCode::Backspace));

    assert_eq!(input::value(app.dom(), inp), "12");
}

#[test]
fn typing_in_text_input_is_unaffected_by_number_filter() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "value", "x").unwrap();
    dom.append_child(root, inp).unwrap();

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    let t = app
        .dom()
        .node(inp)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 1))));

    app.handle_event(key(KeyCode::Char('y')));

    assert_eq!(input::value(app.dom(), inp), "xy");
}

// ── Up/Down stepping ──────────────────────────────────────────────

#[test]
fn up_arrow_steps_value_up_by_default_step_one() {
    let (mut app, inp, _t) = number_input_app("5");
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(input::value(app.dom(), inp), "6");
}

#[test]
fn down_arrow_steps_value_down_by_default_step_one() {
    let (mut app, inp, _t) = number_input_app("5");
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Down));
    assert_eq!(input::value(app.dom(), inp), "4");
}

#[test]
fn up_arrow_uses_step_attribute() {
    let (mut app, inp, _t) = number_input_app("0");
    app.dom_mut().set_attribute(inp, "step", "10").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(input::value(app.dom(), inp), "10");
}

#[test]
fn up_arrow_clamps_to_max() {
    let (mut app, inp, _t) = number_input_app("9");
    app.dom_mut().set_attribute(inp, "max", "10").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    app.handle_event(key(KeyCode::Up));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(input::value(app.dom(), inp), "10");
}

#[test]
fn down_arrow_clamps_to_min() {
    let (mut app, inp, _t) = number_input_app("1");
    app.dom_mut().set_attribute(inp, "min", "0").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Down));
    app.handle_event(key(KeyCode::Down));
    app.handle_event(key(KeyCode::Down));
    assert_eq!(input::value(app.dom(), inp), "0");
}

#[test]
fn empty_value_steps_from_zero() {
    let (mut app, inp, _t) = number_input_app("");
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(input::value(app.dom(), inp), "1");
}

#[test]
fn arrow_does_not_step_when_readonly() {
    let (mut app, inp, _t) = number_input_app("5");
    app.dom_mut().set_attribute(inp, "readonly", "").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(input::value(app.dom(), inp), "5");
}

#[test]
fn arrow_with_modifier_does_not_step() {
    let (mut app, inp, _t) = number_input_app("5");
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Up,
        KeyModifiers::SHIFT,
    )));
    assert_eq!(input::value(app.dom(), inp), "5");
}

#[test]
fn step_fires_change_event() {
    use rdom_core::ListenerOptions;
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut app, inp, _t) = number_input_app("5");
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(inp, "change", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(fired.get(), 1);
}
