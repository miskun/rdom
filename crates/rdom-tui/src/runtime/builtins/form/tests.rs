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
