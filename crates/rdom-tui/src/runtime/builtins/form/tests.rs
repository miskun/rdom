//! `<form>` submit + reset + collect tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, Position, Selection};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::form;
use crate::style::{Stylesheet, TuiStyle};

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn click(x: u16, y: u16) -> Vec<CtEvent> {
    vec![
        CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }),
        CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }),
    ]
}

fn test_app(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

// ── Submit triggers ───────────────────────────────────────────────

#[test]
fn click_on_input_type_submit_fires_submit_event() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert_eq!(fired.get(), 1);
}

#[test]
fn click_on_button_with_no_type_submits_form() {
    // HTML rule: `<button>` without explicit `type` defaults to
    // `type="submit"` when inside a form.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("button");
    let label = dom.create_text_node("Go");
    dom.append_child(btn, label).unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "button",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert!(fired.get());
}

#[test]
fn click_on_button_type_button_does_not_submit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("button");
    dom.set_attribute(btn, "type", "button").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "button",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert!(!fired.get());
}

#[test]
fn submit_event_is_cancelable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);

    // Listener prevents default, then a second listener observes
    // that the first listener already saw the event (preventDefault
    // doesn't stop propagation).
    let saw_after_prevent = Rc::new(Cell::new(false));
    let s = saw_after_prevent.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
            assert!(ctx.event.default_prevented());
            s.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert!(saw_after_prevent.get());
}

#[test]
fn disabled_submit_button_does_not_fire_submit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.set_attribute(btn, "disabled", "").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert!(!fired.get());
}

// ── Implicit Enter submission ─────────────────────────────────────

#[test]
fn enter_in_lone_text_input_submits_form() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(root, form).unwrap();

    let mut app = test_app(dom, Stylesheet::new());
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(inp));
    let t = app
        .dom()
        .node(inp)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));

    app.handle_event(key(KeyCode::Enter));
    assert!(fired.get());
}

#[test]
fn enter_in_form_with_multiple_text_inputs_does_not_submit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let i1 = dom.create_element("input");
    dom.set_attribute(i1, "type", "text").unwrap();
    let i2 = dom.create_element("input");
    dom.set_attribute(i2, "type", "text").unwrap();
    dom.append_child(form, i1).unwrap();
    dom.append_child(form, i2).unwrap();
    dom.append_child(root, form).unwrap();

    let mut app = test_app(dom, Stylesheet::new());
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(i1));
    let t = app
        .dom()
        .node(i1)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));

    app.handle_event(key(KeyCode::Enter));
    assert!(!fired.get());
}

#[test]
fn enter_outside_form_does_not_submit_anything() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.append_child(root, inp).unwrap();

    let mut app = test_app(dom, Stylesheet::new());
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(root, "submit", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(inp));
    let t = app
        .dom()
        .node(inp)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));

    app.handle_event(key(KeyCode::Enter));
    assert!(!fired.get());
}

// ── Reset trigger ─────────────────────────────────────────────────

#[test]
fn click_on_reset_button_fires_reset_event() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "reset").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=reset]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(form, "reset", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }
    assert!(fired.get());
}

// ── collect() helper ──────────────────────────────────────────────

#[test]
fn collect_returns_text_input_value() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "name", "user").unwrap();
    dom.set_attribute(inp, "value", "alice").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    let collected = form::collect(app.dom(), form);
    assert_eq!(collected, vec![("user".to_string(), "alice".to_string())]);
}

#[test]
fn collect_skips_inputs_without_name() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "value", "secret").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    assert!(form::collect(app.dom(), form).is_empty());
}

#[test]
fn collect_skips_disabled_inputs() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "name", "x").unwrap();
    dom.set_attribute(inp, "value", "v").unwrap();
    dom.set_attribute(inp, "disabled", "").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    assert!(form::collect(app.dom(), form).is_empty());
}

#[test]
fn collect_includes_checked_checkboxes_only() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let cb1 = dom.create_element("input");
    dom.set_attribute(cb1, "type", "checkbox").unwrap();
    dom.set_attribute(cb1, "name", "a").unwrap();
    dom.set_attribute(cb1, "value", "1").unwrap();
    dom.set_attribute(cb1, "checked", "").unwrap();
    let cb2 = dom.create_element("input");
    dom.set_attribute(cb2, "type", "checkbox").unwrap();
    dom.set_attribute(cb2, "name", "b").unwrap();
    dom.set_attribute(cb2, "value", "2").unwrap();
    // cb2 NOT checked.
    dom.append_child(form, cb1).unwrap();
    dom.append_child(form, cb2).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    assert_eq!(
        form::collect(app.dom(), form),
        vec![("a".to_string(), "1".to_string())]
    );
}

#[test]
fn collect_default_checkbox_value_is_on() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.set_attribute(cb, "name", "agree").unwrap();
    dom.set_attribute(cb, "checked", "").unwrap();
    dom.append_child(form, cb).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    assert_eq!(
        form::collect(app.dom(), form),
        vec![("agree".to_string(), "on".to_string())]
    );
}

#[test]
fn collect_includes_textarea_text_content() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let ta = dom.create_element("textarea");
    dom.set_attribute(ta, "name", "comments").unwrap();
    let t = dom.create_text_node("hello\nworld");
    dom.append_child(ta, t).unwrap();
    dom.append_child(form, ta).unwrap();
    dom.append_child(root, form).unwrap();

    let app = test_app(dom, Stylesheet::new());
    assert_eq!(
        form::collect(app.dom(), form),
        vec![("comments".to_string(), "hello\nworld".to_string())]
    );
}

// ── End-to-end: collect on submit ─────────────────────────────────

#[test]
fn submit_handler_can_read_form_data_via_collect() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.set_attribute(inp, "name", "q").unwrap();
    dom.set_attribute(inp, "value", "rust").unwrap();
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "input[type=submit]",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "input[type=text]",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        );
    let mut app = test_app(dom, sheet);
    let captured: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
    let c = captured.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
            ctx.event.prevent_default();
            *c.borrow_mut() = form::collect(ctx.dom, form);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    // The text input is at row 0 (height 1); submit button is at row 1.
    for ev in click(1, 1) {
        app.handle_event(ev);
    }
    assert_eq!(
        *captured.borrow(),
        vec![("q".to_string(), "rust".to_string())]
    );
}

// ── Step 5: typed submit event detail ─────────────────────────────

#[test]
fn submit_event_carries_submitter_on_button_click() {
    // Canonical step-5 failing test: clicking <input type=submit>
    // fires submit with EventDetail::Submit { submitter: Some(btn) }.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);

    let captured: Rc<Cell<Option<Option<rdom_core::NodeId>>>> = Rc::new(Cell::new(None));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
                let detail = ctx
                    .event
                    .detail
                    .as_submit()
                    .expect("submit must carry EventDetail::Submit");
                captured.set(Some(detail.submitter));
            })
            .unwrap();
    }
    app.draw_if_dirty().unwrap();
    for ev in click(1, 0) {
        app.handle_event(ev);
    }

    let seen = captured.get().expect("submit listener fired");
    assert_eq!(seen, Some(btn));
}

#[test]
fn implicit_enter_submit_has_no_submitter() {
    // HTML rule: pressing Enter in a lone-text-input form fires
    // submit with submitter=None (no clicked button).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "text").unwrap();
    dom.append_child(form, inp).unwrap();
    dom.append_child(root, form).unwrap();

    let mut app = test_app(dom, Stylesheet::new());

    let captured: Rc<Cell<Option<Option<rdom_core::NodeId>>>> = Rc::new(Cell::new(None));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
                let detail = ctx
                    .event
                    .detail
                    .as_submit()
                    .expect("submit must carry EventDetail::Submit");
                captured.set(Some(detail.submitter));
            })
            .unwrap();
    }
    app.dom_mut().set_focused(Some(inp));
    let t = app
        .dom()
        .node(inp)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));

    app.handle_event(key(KeyCode::Enter));

    let seen = captured.get().expect("submit listener fired");
    assert!(
        seen.is_none(),
        "implicit Enter submit must have submitter=None; got {seen:?}"
    );
}

// ── FORM-DEFAULTS-1: defaults survive edits; reset restores them ────

/// `defaultValue` / `defaultChecked` are kept apart from the live
/// state, and a `<form>` reset restores every control to them: the
/// text input's value, the checkbox, and the radio group's original
/// member.
#[test]
fn reset_restores_default_value_and_default_checked() {
    use crate::accessors::TuiAccessors;
    use crate::runtime::builtins::input;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let text = dom.create_element("input");
    dom.set_attribute(text, "value", "a").unwrap();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    let ra = dom.create_element("input");
    dom.set_attribute(ra, "type", "radio").unwrap();
    dom.set_attribute(ra, "name", "g").unwrap();
    dom.set_attribute(ra, "checked", "").unwrap();
    let rb = dom.create_element("input");
    dom.set_attribute(rb, "type", "radio").unwrap();
    dom.set_attribute(rb, "name", "g").unwrap();
    let reset = dom.create_element("input");
    dom.set_attribute(reset, "type", "reset").unwrap();
    for c in [text, cb, ra, rb, reset] {
        dom.append_child(form, c).unwrap();
    }
    dom.append_child(root, form).unwrap();
    // One control per row so the clicks are unambiguous. The typed
    // selectors outrank the UA sheet's `input[type=…]` inline-block rules.
    let mut sheet = Stylesheet::new().rule_unchecked(
        "form",
        TuiStyle::new().direction(crate::layout::Direction::Column),
    );
    for sel in [
        "input",
        "input[type=checkbox]",
        "input[type=radio]",
        "input[type=reset]",
    ] {
        sheet = sheet.rule_unchecked(
            sel,
            TuiStyle::new()
                .display(crate::layout::Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        );
    }
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();

    // Change every control.
    input::set_value(app.dom_mut(), text, "ax");
    for ev in click(1, 1) {
        app.handle_event(ev); // checkbox on
    }
    for ev in click(1, 3) {
        app.handle_event(ev); // radio b
    }
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(text).value(), Some("ax".into()));
    assert!(app.dom().node(cb).checked());
    assert!(!app.dom().node(ra).checked() && app.dom().node(rb).checked());
    // The defaults are what the controls were authored with.
    assert_eq!(app.dom().node(text).default_value(), Some("a".into()));
    assert_eq!(app.dom().node(cb).default_checked(), Some(false));
    assert_eq!(app.dom().node(ra).default_checked(), Some(true));

    for ev in click(1, 4) {
        app.handle_event(ev); // reset
    }
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(text).value(), Some("a".into()));
    assert_eq!(app.dom().node(text).get_attribute("value"), Some("a"));
    assert!(!app.dom().node(cb).checked());
    assert!(app.dom().node(ra).checked() && !app.dom().node(rb).checked());
}

// ── P6G-FORM-RESET-1: select, range, textarea, default setters ──────

/// A form with `controls` and a trailing reset button, built into an
/// App (seeded + listeners installed). Returns the app and the reset
/// button.
fn form_app(
    build: impl FnOnce(&mut TuiDom, rdom_core::NodeId),
) -> (App<TestBackend>, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    build(&mut dom, form);
    let reset = dom.create_element("input");
    dom.set_attribute(reset, "type", "reset").unwrap();
    dom.append_child(form, reset).unwrap();
    let mut app = test_app(dom, Stylesheet::new());
    app.draw_if_dirty().unwrap();
    (app, reset)
}

fn click_reset(app: &mut App<TestBackend>, reset: rdom_core::NodeId) {
    use crate::accessors::TuiAccessorsMut;
    app.dom_mut().node_mut(reset).click();
}

fn option(
    dom: &mut TuiDom,
    select: rdom_core::NodeId,
    value: &str,
    selected: bool,
) -> rdom_core::NodeId {
    let o = dom.create_element("option");
    dom.set_attribute(o, "value", value).unwrap();
    let t = dom.create_text_node(value);
    dom.append_child(o, t).unwrap();
    if selected {
        dom.set_attribute(o, "selected", "").unwrap();
    }
    dom.append_child(select, o).unwrap();
    o
}

/// HTML §4.10.7 reset algorithm for `<select>`: every `<option>` goes
/// back to its `defaultSelected` (the authored `selected` attribute).
#[test]
fn reset_restores_select_options_to_default_selected() {
    use crate::accessors::{TuiAccessors, TuiAccessorsMut};
    let mut ids = Vec::new();
    let (mut app, reset) = form_app(|dom, form| {
        let single = dom.create_element("select");
        dom.append_child(form, single).unwrap();
        ids.push(single);
        ids.push(option(dom, single, "a", false));
        ids.push(option(dom, single, "b", true));
        let multi = dom.create_element("select");
        dom.set_attribute(multi, "multiple", "").unwrap();
        dom.append_child(form, multi).unwrap();
        ids.push(multi);
        ids.push(option(dom, multi, "x", true));
        ids.push(option(dom, multi, "y", false));
    });
    let [single, a, b, multi, x, y] = ids[..] else {
        unreachable!()
    };
    app.dom_mut().node_mut(single).set_value("a").unwrap();
    app.dom_mut().node_mut(multi).set_value("y").unwrap();
    assert_eq!(app.dom().node(single).value(), Some("a".into()));
    assert_eq!(app.dom().node(multi).value(), Some("y".into()));

    click_reset(&mut app, reset);
    assert!(!app.dom().node(a).has_attribute("selected"));
    assert!(app.dom().node(b).has_attribute("selected"));
    assert!(app.dom().node(x).has_attribute("selected"));
    assert!(!app.dom().node(y).has_attribute("selected"));
    assert_eq!(app.dom().node(single).value(), Some("b".into()));
}

/// A slider's `defaultValue` is its `value` content attribute; reset
/// writes it back (removing the attribute when there was none, which
/// puts the thumb back at the midpoint).
#[test]
fn reset_restores_range_to_its_default_value() {
    use crate::accessors::TuiAccessors;
    use crate::runtime::builtins::range;
    let mut ids = Vec::new();
    let (mut app, reset) = form_app(|dom, form| {
        for value in [Some("30"), None] {
            let r = dom.create_element("input");
            dom.set_attribute(r, "type", "range").unwrap();
            if let Some(v) = value {
                dom.set_attribute(r, "value", v).unwrap();
            }
            dom.append_child(form, r).unwrap();
            ids.push(r);
        }
    });
    let [r30, rmid] = ids[..] else { unreachable!() };
    range::set_value(app.dom_mut(), r30, 70.0);
    range::set_value(app.dom_mut(), rmid, 10.0);
    assert_eq!(app.dom().node(r30).default_value(), Some("30".into()));
    assert_eq!(app.dom().node(rmid).default_value(), Some("".into()));

    click_reset(&mut app, reset);
    assert_eq!(app.dom().node(r30).get_attribute("value"), Some("30"));
    assert_eq!(app.dom().node(rmid).get_attribute("value"), None);
    assert_eq!(range::value_of(app.dom(), rmid), 50.0);
}

/// `TuiAccessorsMut::set_value` on a `<textarea>` is `.value =`: it
/// leaves `defaultValue` (the authored text) alone.
#[test]
fn textarea_set_value_keeps_the_authored_default_value() {
    use crate::accessors::{TuiAccessors, TuiAccessorsMut};
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    let t = dom.create_text_node("orig");
    dom.append_child(ta, t).unwrap();
    dom.append_child(root, ta).unwrap();
    dom.node_mut(ta).set_value("new").unwrap();
    assert_eq!(dom.node(ta).value(), Some("new".into()));
    assert_eq!(dom.node(ta).default_value(), Some("orig".into()));
}

/// HTML: `defaultValue` of an `<input>` whose value mode is not
/// "value" (hidden, submit, checkbox, …) — and of any input before a
/// change — is its `value` content attribute.
#[test]
fn default_value_of_non_text_inputs_is_the_value_attribute() {
    use crate::accessors::TuiAccessors;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut make = |ty: &str, value: &str| {
        let i = dom.create_element("input");
        dom.set_attribute(i, "type", ty).unwrap();
        dom.set_attribute(i, "value", value).unwrap();
        dom.append_child(root, i).unwrap();
        i
    };
    let range = make("range", "30");
    let hidden = make("hidden", "h");
    let submit = make("submit", "Go");
    let cb = make("checkbox", "yes");
    assert_eq!(dom.node(range).default_value(), Some("30".into()));
    assert_eq!(dom.node(hidden).default_value(), Some("h".into()));
    assert_eq!(dom.node(submit).default_value(), Some("Go".into()));
    assert_eq!(dom.node(cb).default_value(), Some("yes".into()));
}

/// `defaultValue = …` / `defaultChecked = …`: the new default is what
/// a reset restores; the live state is left alone.
#[test]
fn default_setters_change_what_reset_restores() {
    use crate::accessors::{TuiAccessors, TuiAccessorsMut};
    let mut ids = Vec::new();
    let (mut app, reset) = form_app(|dom, form| {
        let text = dom.create_element("input");
        dom.set_attribute(text, "value", "a").unwrap();
        let cb = dom.create_element("input");
        dom.set_attribute(cb, "type", "checkbox").unwrap();
        let ta = dom.create_element("textarea");
        let hidden = dom.create_element("input");
        dom.set_attribute(hidden, "type", "hidden").unwrap();
        for c in [text, cb, ta, hidden] {
            dom.append_child(form, c).unwrap();
            ids.push(c);
        }
    });
    let [text, cb, ta, hidden] = ids[..] else {
        unreachable!()
    };
    let dom = app.dom_mut();
    dom.node_mut(text).set_default_value("z").unwrap();
    dom.node_mut(ta).set_default_value("body").unwrap();
    dom.node_mut(cb).set_default_checked(true).unwrap();
    dom.node_mut(hidden).set_default_value("q").unwrap();
    assert_eq!(
        dom.node(text).value(),
        Some("a".into()),
        "live value untouched"
    );
    assert!(!dom.node(cb).checked(), "live checkedness untouched");
    assert_eq!(dom.node(text).default_value(), Some("z".into()));
    assert_eq!(dom.node(cb).default_checked(), Some(true));
    // A hidden input's value *is* its default (value mode "default").
    assert_eq!(dom.node(hidden).get_attribute("value"), Some("q"));

    click_reset(&mut app, reset);
    let dom = app.dom();
    assert_eq!(dom.node(text).value(), Some("z".into()));
    assert_eq!(dom.node(ta).value(), Some("body".into()));
    assert!(dom.node(cb).checked());
}

// ── P6G-FORM-COLLECT-DISABLED-1: disabled options are not submitted ──

/// HTML §4.10.21.4 "constructing the entry list": a `<select>` appends
/// one entry per option that is selected **and not disabled**. An
/// option is disabled by its own attribute or by a disabled
/// `<optgroup>` parent (HTML §4.10.10).
#[test]
fn collect_skips_selected_options_that_are_disabled() {
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        let multi = dom.create_element("select");
        dom.set_attribute(multi, "name", "m").unwrap();
        dom.set_attribute(multi, "multiple", "").unwrap();
        dom.append_child(form, multi).unwrap();
        option(dom, multi, "kept", true);
        let own = option(dom, multi, "own-disabled", true);
        dom.set_attribute(own, "disabled", "").unwrap();
        let group = dom.create_element("optgroup");
        dom.set_attribute(group, "disabled", "").unwrap();
        dom.append_child(multi, group).unwrap();
        option(dom, group, "group-disabled", true);
    });
    assert_eq!(
        form::collect(app.dom(), form_id.unwrap()),
        vec![("m".to_string(), "kept".to_string())]
    );
}

/// A single-select whose only selected option is disabled submits
/// nothing (the selectedness algorithm leaves an explicit selection
/// alone, so the disabled option stays selected).
#[test]
fn collect_single_select_with_disabled_selected_option_submits_nothing() {
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        let single = dom.create_element("select");
        dom.set_attribute(single, "name", "s").unwrap();
        dom.append_child(form, single).unwrap();
        option(dom, single, "a", false);
        let b = option(dom, single, "b", true);
        dom.set_attribute(b, "disabled", "").unwrap();
    });
    assert!(form::collect(app.dom(), form_id.unwrap()).is_empty());
}

// ── P6G-FORM-SUBMITTER-1: only the submitter contributes a button entry ──

/// Focus `input`, park the caret in it and press Enter.
fn press_enter_in(app: &mut App<TestBackend>, input: rdom_core::NodeId) {
    app.dom_mut().set_focused(Some(input));
    let t = app
        .dom()
        .node(input)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));
    app.handle_event(key(KeyCode::Enter));
}

fn named(
    dom: &mut TuiDom,
    form: rdom_core::NodeId,
    tag: &str,
    attrs: &[(&str, &str)],
) -> rdom_core::NodeId {
    let el = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(el, k, v).unwrap();
    }
    dom.append_child(form, el).unwrap();
    el
}

/// Records every `submit` on `form` as its `SubmitEvent.submitter` and
/// the entry list built for that submitter.
type Submissions = Rc<RefCell<Vec<(Option<rdom_core::NodeId>, Vec<(String, String)>)>>>;

fn record_submissions(app: &mut App<TestBackend>, form: rdom_core::NodeId) -> Submissions {
    let log: Submissions = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
            ctx.event.prevent_default();
            let submitter = ctx
                .event
                .detail
                .as_submit()
                .expect("submit detail")
                .submitter;
            let entries = form::collect_with_submitter(ctx.dom, form, submitter);
            l.borrow_mut().push((submitter, entries));
        })
        .unwrap();
    log
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
}

/// HTML §4.10.21.4: a button contributes an entry only when it is the
/// submitter; `new FormData(form)` (no submitter) has no button entry at
/// all, and reset / plain buttons never contribute.
#[test]
fn collect_includes_only_the_submitter_among_buttons() {
    let mut ids = Vec::new();
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        named(dom, form, "input", &[("name", "q"), ("value", "rust")]);
        ids.push(named(
            dom,
            form,
            "input",
            &[("type", "submit"), ("name", "a"), ("value", "A")],
        ));
        ids.push(named(dom, form, "button", &[("name", "b"), ("value", "B")]));
        ids.push(named(
            dom,
            form,
            "input",
            &[("type", "reset"), ("name", "r"), ("value", "R")],
        ));
        ids.push(named(
            dom,
            form,
            "input",
            &[("type", "button"), ("name", "x"), ("value", "X")],
        ));
        ids.push(named(
            dom,
            form,
            "button",
            &[("type", "button"), ("name", "y"), ("value", "Y")],
        ));
    });
    let form = form_id.unwrap();
    let dom = app.dom();
    assert_eq!(form::collect(dom, form), pairs(&[("q", "rust")]));
    assert_eq!(
        form::collect_with_submitter(dom, form, Some(ids[0])),
        pairs(&[("q", "rust"), ("a", "A")])
    );
    assert_eq!(
        form::collect_with_submitter(dom, form, Some(ids[1])),
        pairs(&[("q", "rust"), ("b", "B")])
    );
    for &not_submit in &ids[2..] {
        assert_eq!(
            form::collect_with_submitter(dom, form, Some(not_submit)),
            pairs(&[("q", "rust")]),
            "a non-submit button never contributes"
        );
    }
}

/// A submitter without a `value` attribute contributes the empty string;
/// one without a `name` contributes nothing.
#[test]
fn submitter_without_value_contributes_empty_string_and_without_name_nothing() {
    let mut ids = Vec::new();
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        ids.push(named(dom, form, "button", &[("name", "go")]));
        ids.push(named(
            dom,
            form,
            "input",
            &[("type", "submit"), ("value", "unnamed")],
        ));
    });
    let form = form_id.unwrap();
    assert_eq!(
        form::collect_with_submitter(app.dom(), form, Some(ids[0])),
        pairs(&[("go", "")])
    );
    assert!(form::collect_with_submitter(app.dom(), form, Some(ids[1])).is_empty());
}

/// P6G-SUBMIT-DEFAULT-LABEL-1: a value-less `<input type=submit>`
/// submitter contributes `""` — HTML's entry list takes the element's
/// value (value mode *default*: the attribute or `""`), not the
/// implementation-defined "Submit" label (DIVERGENCES).
#[test]
fn value_less_submit_input_submitter_contributes_empty_string() {
    let mut ids = Vec::new();
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        ids.push(named(
            dom,
            form,
            "input",
            &[("type", "submit"), ("name", "a")],
        ));
    });
    let form = form_id.unwrap();
    assert_eq!(
        form::collect_with_submitter(app.dom(), form, Some(ids[0])),
        pairs(&[("a", "")])
    );
}

/// Clicking the second of two named submit buttons submits with it as
/// the submitter, and only its entry is in the list.
#[test]
fn clicked_submit_button_is_the_submitter_and_the_only_button_entry() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    named(
        &mut dom,
        form,
        "input",
        &[("type", "submit"), ("name", "act"), ("value", "save")],
    );
    let second = named(
        &mut dom,
        form,
        "input",
        &[("type", "submit"), ("name", "act"), ("value", "delete")],
    );
    let mut app = test_app(dom, Stylesheet::new());
    let log = record_submissions(&mut app, form);
    use crate::accessors::TuiAccessorsMut;
    app.dom_mut().node_mut(second).click();
    assert_eq!(
        *log.borrow(),
        vec![(Some(second), pairs(&[("act", "delete")]))]
    );
}

/// HTML §4.10.21.2 implicit submission: when the form has a default
/// button (its first submit button in tree order), Enter fires a
/// `click` at it — so the default button is the submitter, and the
/// "only one field blocks implicit submission" rule does not apply.
#[test]
fn enter_with_a_default_button_clicks_it_and_submits_with_it() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    let q = named(&mut dom, form, "input", &[("name", "q"), ("value", "rust")]);
    named(&mut dom, form, "input", &[("name", "r"), ("value", "tui")]);
    named(
        &mut dom,
        form,
        "button",
        &[("type", "button"), ("name", "n"), ("value", "N")],
    );
    named(&mut dom, form, "input", &[("type", "reset")]);
    let default = named(
        &mut dom,
        form,
        "button",
        &[("type", "submit"), ("name", "go"), ("value", "1")],
    );
    named(
        &mut dom,
        form,
        "input",
        &[("type", "submit"), ("name", "later"), ("value", "2")],
    );
    let mut app = test_app(dom, Stylesheet::new());
    let log = record_submissions(&mut app, form);
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let c = clicks.clone();
    app.dom_mut()
        .add_event_listener(form, "click", ListenerOptions::default(), move |ctx| {
            c.borrow_mut().push(ctx.event.target);
        })
        .unwrap();
    press_enter_in(&mut app, q);
    assert_eq!(*clicks.borrow(), vec![Some(default)]);
    assert_eq!(
        *log.borrow(),
        vec![(
            Some(default),
            pairs(&[("q", "rust"), ("r", "tui"), ("go", "1")])
        )]
    );
}

/// HTML: if the default button is disabled, implicit submission does
/// nothing — even in a form with a single text field.
#[test]
fn enter_with_a_disabled_default_button_does_not_submit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    let q = named(&mut dom, form, "input", &[("name", "q")]);
    named(
        &mut dom,
        form,
        "input",
        &[("type", "submit"), ("disabled", "")],
    );
    named(&mut dom, form, "input", &[("type", "submit")]);
    let mut app = test_app(dom, Stylesheet::new());
    let log = record_submissions(&mut app, form);
    press_enter_in(&mut app, q);
    assert!(log.borrow().is_empty());
}

/// HTML §4.10.6: a `<button>` whose `type` is missing **or invalid** is
/// in the Submit Button state.
#[test]
fn button_with_an_invalid_type_is_a_submit_button() {
    let mut ids = Vec::new();
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        ids.push(named(
            dom,
            form,
            "button",
            &[("type", "bogus"), ("name", "a"), ("value", "A")],
        ));
        ids.push(named(
            dom,
            form,
            "button",
            &[("type", "reset"), ("name", "b"), ("value", "B")],
        ));
    });
    let form = form_id.unwrap();
    let dom = app.dom();
    assert_eq!(
        form::collect_with_submitter(dom, form, Some(ids[0])),
        pairs(&[("a", "A")])
    );
    assert!(form::collect_with_submitter(dom, form, Some(ids[1])).is_empty());
}

/// Clicking a `<button type="bogus">` submits its form.
#[test]
fn click_on_button_with_invalid_type_submits() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    let btn = named(&mut dom, form, "button", &[("type", "bogus")]);
    let mut app = test_app(dom, Stylesheet::new());
    let log = record_submissions(&mut app, form);
    use crate::accessors::TuiAccessorsMut;
    app.dom_mut().node_mut(btn).click();
    assert_eq!(*log.borrow(), vec![(Some(btn), vec![])]);
}

// ── P7-FIELDSET-DISABLED-1: a disabled fieldset disables its controls ──

/// HTML §4.10.18.5 / §4.10.21.4: a control inside a `<fieldset
/// disabled>` is disabled and is not submitted; one inside the
/// fieldset's first `<legend>` stays enabled and is.
#[test]
fn collect_skips_controls_in_a_disabled_fieldset_but_not_its_first_legend() {
    let mut form_id = None;
    let (app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        named(dom, form, "input", &[("name", "a"), ("value", "A")]);
        let fs = named(dom, form, "fieldset", &[("disabled", "")]);
        let legend = named(dom, fs, "legend", &[]);
        named(dom, legend, "input", &[("name", "l"), ("value", "L")]);
        named(dom, fs, "input", &[("name", "b"), ("value", "B")]);
        named(
            dom,
            fs,
            "input",
            &[("type", "checkbox"), ("name", "c"), ("checked", "")],
        );
        let inner = named(dom, fs, "fieldset", &[]);
        named(dom, inner, "input", &[("name", "d"), ("value", "D")]);
    });
    assert_eq!(
        form::collect(app.dom(), form_id.unwrap()),
        pairs(&[("a", "A"), ("l", "L")])
    );
}

/// A submit button inside a disabled fieldset does not submit when
/// clicked, and as the form's default button blocks implicit submission.
#[test]
fn submit_button_in_a_disabled_fieldset_does_not_submit() {
    let mut form_id = None;
    let mut input_id = None;
    let mut button_id = None;
    let (mut app, _reset) = form_app(|dom, form| {
        form_id = Some(form);
        input_id = Some(named(dom, form, "input", &[("value", "q")]));
        let fs = named(dom, form, "fieldset", &[("disabled", "")]);
        button_id = Some(named(dom, fs, "button", &[("name", "go")]));
    });
    let log = record_submissions(&mut app, form_id.unwrap());
    click_reset(&mut app, button_id.unwrap()); // `.click()` on the button
    assert!(
        log.borrow().is_empty(),
        "a disabled-by-fieldset button does not submit"
    );
    press_enter_in(&mut app, input_id.unwrap());
    assert!(
        log.borrow().is_empty(),
        "a disabled-by-fieldset default button blocks implicit submission"
    );
}

// ── P7-FORM-OWNER-1: the `form` attribute and the submitter overrides ──

/// Builds a tree under the root with `build` into a seeded App.
fn owner_app(build: impl FnOnce(&mut TuiDom, rdom_core::NodeId)) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    build(&mut dom, root);
    let mut app = test_app(dom, Stylesheet::new());
    app.draw_if_dirty().unwrap();
    app
}

/// HTML §4.10.17.3: a control outside the form that names it with
/// `form="f"` belongs to it — submitted and reset with it, in tree order
/// of the whole document; `form::elements` lists it.
#[test]
fn control_outside_the_form_with_a_form_attribute_is_submitted_and_reset_with_it() {
    let mut ids = Vec::new();
    let mut app = owner_app(|dom, root| {
        ids.push(named(
            dom,
            root,
            "input",
            &[("name", "before"), ("form", "f"), ("value", "1")],
        ));
        let form = named(dom, root, "form", &[("id", "f")]);
        ids.push(form);
        ids.push(named(
            dom,
            form,
            "input",
            &[("name", "inside"), ("value", "2")],
        ));
        ids.push(named(
            dom,
            root,
            "input",
            &[("name", "after"), ("form", "f"), ("value", "3")],
        ));
        named(dom, root, "input", &[("name", "stray"), ("value", "4")]);
        ids.push(named(
            dom,
            root,
            "input",
            &[("type", "reset"), ("form", "f")],
        ));
    });
    let (before, form, inside, after, reset) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
    assert_eq!(
        form::elements(app.dom(), form),
        vec![before, inside, after, reset]
    );
    {
        use crate::accessors::TuiAccessors;
        assert_eq!(app.dom().node(before).input_form(), Some(form));
        assert_eq!(app.dom().node(reset).input_form(), Some(form));
    }
    assert_eq!(
        form::collect(app.dom(), form),
        pairs(&[("before", "1"), ("inside", "2"), ("after", "3")])
    );

    {
        use crate::accessors::TuiAccessorsMut;
        app.dom_mut().node_mut(before).set_value("changed").unwrap();
        app.dom_mut().node_mut(after).set_value("changed").unwrap();
    }
    click_reset(&mut app, reset);
    assert_eq!(
        form::collect(app.dom(), form),
        pairs(&[("before", "1"), ("inside", "2"), ("after", "3")]),
        "the reset button outside the form resets its owner's controls"
    );
}

/// A control inside form A with `form="b"` belongs to B; a submit button
/// inside A with `form="b"` submits B.
#[test]
fn control_inside_one_form_can_belong_to_another() {
    let mut ids = Vec::new();
    let mut app = owner_app(|dom, root| {
        let a = named(dom, root, "form", &[("id", "a")]);
        let b = named(dom, root, "form", &[("id", "b")]);
        named(dom, a, "input", &[("name", "for_a"), ("value", "A")]);
        named(
            dom,
            a,
            "input",
            &[("name", "for_b"), ("form", "b"), ("value", "B")],
        );
        let go = named(dom, a, "button", &[("form", "b"), ("name", "go")]);
        ids.extend([a, b, go]);
    });
    let (a, b, go) = (ids[0], ids[1], ids[2]);
    assert_eq!(form::collect(app.dom(), a), pairs(&[("for_a", "A")]));
    assert_eq!(form::collect(app.dom(), b), pairs(&[("for_b", "B")]));

    let log_a = record_submissions(&mut app, a);
    let log_b = record_submissions(&mut app, b);
    click_reset(&mut app, go); // `.click()` on the submit button
    assert!(log_a.borrow().is_empty(), "form A is not submitted");
    assert_eq!(
        *log_b.borrow(),
        vec![(Some(go), pairs(&[("for_b", "B"), ("go", "")]))]
    );
}

/// HTML §4.10.17.3: a `form` attribute that names no `<form>` means no
/// owner — even for a control nested inside a form.
#[test]
fn unknown_form_attribute_means_no_owner_even_inside_a_form() {
    let mut ids = Vec::new();
    let mut app = owner_app(|dom, root| {
        let f = named(dom, root, "form", &[("id", "f")]);
        named(dom, f, "input", &[("name", "ok"), ("value", "1")]);
        named(
            dom,
            f,
            "input",
            &[("name", "lost"), ("form", "nope"), ("value", "2")],
        );
        let go = named(dom, f, "button", &[("form", "nope")]);
        ids.extend([f, go]);
    });
    let (f, go) = (ids[0], ids[1]);
    assert_eq!(form::collect(app.dom(), f), pairs(&[("ok", "1")]));
    let log = record_submissions(&mut app, f);
    click_reset(&mut app, go);
    assert!(
        log.borrow().is_empty(),
        "an unowned submit button submits nothing"
    );
}

/// Implicit submission uses the focused input's form owner and that
/// form's default button, both through `form=`.
#[test]
fn implicit_submission_follows_the_form_attribute() {
    let mut ids = Vec::new();
    let mut app = owner_app(|dom, root| {
        let f = named(dom, root, "form", &[("id", "f")]);
        let input = named(
            dom,
            root,
            "input",
            &[("form", "f"), ("name", "q"), ("value", "x")],
        );
        let go = named(
            dom,
            root,
            "button",
            &[("form", "f"), ("name", "go"), ("value", "G")],
        );
        ids.extend([f, input, go]);
    });
    let (f, input, go) = (ids[0], ids[1], ids[2]);
    let log = record_submissions(&mut app, f);
    press_enter_in(&mut app, input);
    assert_eq!(
        *log.borrow(),
        vec![(Some(go), pairs(&[("q", "x"), ("go", "G")]))]
    );
}

/// The `submit` event reports the effective `action` / `method` /
/// `enctype` / `target` / no-validate state: the form's attributes, or
/// the submitter's `form*` overrides (HTML §4.10.19.6).
#[test]
fn submit_event_reports_the_effective_submission_attributes() {
    use rdom_core::{FormEnctype, FormMethod, SubmitDetail};
    let mut ids = Vec::new();
    let mut app = owner_app(|dom, root| {
        let f = named(
            dom,
            root,
            "form",
            &[
                ("action", "/save"),
                ("method", "post"),
                ("enctype", "multipart/form-data"),
                ("target", "_blank"),
            ],
        );
        let plain = named(dom, f, "button", &[]);
        let over = named(
            dom,
            f,
            "input",
            &[
                ("type", "submit"),
                ("formaction", "/other"),
                ("formmethod", "get"),
                ("formenctype", "text/plain"),
                ("formtarget", "_self"),
                ("formnovalidate", ""),
            ],
        );
        ids.extend([f, plain, over]);
    });
    let (f, plain, over) = (ids[0], ids[1], ids[2]);
    let seen: Rc<RefCell<Vec<SubmitDetail>>> = Rc::new(RefCell::new(Vec::new()));
    let s = seen.clone();
    app.dom_mut()
        .add_event_listener(f, "submit", ListenerOptions::default(), move |ctx| {
            ctx.event.prevent_default();
            s.borrow_mut()
                .push(ctx.event.detail.as_submit().unwrap().clone());
        })
        .unwrap();
    click_reset(&mut app, plain);
    click_reset(&mut app, over);
    let seen = seen.borrow();
    assert_eq!(seen.len(), 2);
    let d = &seen[0];
    assert_eq!(
        (
            d.submitter,
            d.action.as_str(),
            d.method,
            d.enctype,
            d.target.as_str(),
            d.no_validate
        ),
        (
            Some(plain),
            "/save",
            FormMethod::Post,
            FormEnctype::MultipartFormData,
            "_blank",
            false
        )
    );
    let d = &seen[1];
    assert_eq!(
        (
            d.submitter,
            d.action.as_str(),
            d.method,
            d.enctype,
            d.target.as_str(),
            d.no_validate
        ),
        (
            Some(over),
            "/other",
            FormMethod::Get,
            FormEnctype::TextPlain,
            "_self",
            true
        )
    );
}
