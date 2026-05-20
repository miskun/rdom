//! `<dialog>` show/showModal/close + cancel + form-method-dialog
//! integration tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::dialog;
use crate::style::{Stylesheet, TuiStyle};

fn test_app(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let backend = TestBackend::new(40, 8);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

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

/// Build a `<dialog>` parented under root. Returns (app, dialog_id).
fn dialog_app() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.append_child(root, dlg).unwrap();
    let app = test_app(dom, Stylesheet::new());
    (app, dlg)
}

// ── show / showModal / close — pure API ────────────────────────────

#[test]
fn show_sets_open_attribute() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg);
    assert!(app.dom().node(dlg).has_attribute("open"));
    assert!(!dialog::is_modal(app.dom(), dlg));
}

#[test]
fn show_modal_sets_open_and_modal_marker() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg);
    assert!(app.dom().node(dlg).has_attribute("open"));
    assert!(dialog::is_modal(app.dom(), dlg));
}

#[test]
fn show_after_show_modal_clears_modal_marker() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg);
    dialog::show(app.dom_mut(), dlg);
    assert!(app.dom().node(dlg).has_attribute("open"));
    assert!(!dialog::is_modal(app.dom(), dlg));
}

#[test]
fn close_clears_open_attribute_and_stores_return_value() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg);
    dialog::close(app.dom_mut(), dlg, "ok");
    assert!(!app.dom().node(dlg).has_attribute("open"));
    assert_eq!(dialog::return_value(app.dom(), dlg), "ok");
}

#[test]
fn close_fires_close_event_on_dialog() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg);
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(dlg, "close", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    dialog::close(app.dom_mut(), dlg, "");
    assert_eq!(fired.get(), 1);
}

#[test]
fn close_event_does_not_bubble() {
    let (mut app, dlg) = dialog_app();
    let root = app.dom().root();
    dialog::show(app.dom_mut(), dlg);
    let saw = Rc::new(Cell::new(false));
    let s = saw.clone();
    app.dom_mut()
        .add_event_listener(root, "close", ListenerOptions::default(), move |_| {
            s.set(true);
        })
        .unwrap();
    dialog::close(app.dom_mut(), dlg, "");
    assert!(!saw.get(), "close must not bubble past dialog");
}

#[test]
fn close_on_already_closed_dialog_is_noop() {
    let (mut app, dlg) = dialog_app();
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(dlg, "close", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    dialog::close(app.dom_mut(), dlg, "x");
    assert_eq!(fired.get(), 0);
    assert_eq!(dialog::return_value(app.dom(), dlg), "");
}

// ── Esc cancel (modal vs non-modal) ────────────────────────────────

#[test]
fn esc_in_modal_dialog_fires_cancel_then_closes() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg);
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    for ty in ["cancel", "close"] {
        let o = order.clone();
        app.dom_mut()
            .add_event_listener(dlg, ty, ListenerOptions::default(), move |_| {
                o.borrow_mut().push(ty);
            })
            .unwrap();
    }
    app.dom_mut().set_focused(Some(dlg));
    app.handle_event(key(KeyCode::Esc));
    assert_eq!(*order.borrow(), vec!["cancel", "close"]);
    assert!(!app.dom().node(dlg).has_attribute("open"));
}

#[test]
fn esc_in_non_modal_dialog_does_nothing() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg);
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(dlg, "cancel", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(dlg));
    app.handle_event(key(KeyCode::Esc));
    assert!(!fired.get());
    assert!(app.dom().node(dlg).has_attribute("open"));
}

#[test]
fn prevent_default_on_cancel_keeps_modal_dialog_open() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg);
    app.dom_mut()
        .add_event_listener(dlg, "cancel", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    app.dom_mut().set_focused(Some(dlg));
    app.handle_event(key(KeyCode::Esc));
    assert!(app.dom().node(dlg).has_attribute("open"));
}

#[test]
fn esc_with_modifier_does_not_trigger_cancel() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg);
    app.dom_mut().set_focused(Some(dlg));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Esc,
        KeyModifiers::SHIFT,
    )));
    assert!(app.dom().node(dlg).has_attribute("open"));
}

#[test]
fn esc_outside_any_dialog_does_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let mut app = test_app(dom, Stylesheet::new());
    app.dom_mut().set_focused(Some(p));
    // No panic, no crash — that's the whole test.
    app.handle_event(key(KeyCode::Esc));
}

#[test]
fn esc_on_focused_element_inside_modal_dialog_closes_dialog() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let inner = dom.create_element("button");
    dom.append_child(dlg, inner).unwrap();
    dom.append_child(root, dlg).unwrap();
    let mut app = test_app(dom, Stylesheet::new());
    dialog::show_modal(app.dom_mut(), dlg);
    app.dom_mut().set_focused(Some(inner));
    app.handle_event(key(KeyCode::Esc));
    assert!(!app.dom().node(dlg).has_attribute("open"));
}

// ── <form method="dialog"> integration ─────────────────────────────

#[test]
fn form_method_dialog_submit_closes_enclosing_dialog_with_button_value() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let form = dom.create_element("form");
    dom.set_attribute(form, "method", "dialog").unwrap();
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.set_attribute(btn, "value", "confirm").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(dlg, form).unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    dialog::show_modal(app.dom_mut(), dlg);
    app.draw_if_dirty().unwrap();

    // UA `<dialog>` chrome: 1-cell border + padding 1 2. The
    // submit button sits at the dialog's content-area origin
    // (col = border + padding-left = 3, row = border + padding-
    // top = 2). Click at col 5 row 2 lands inside the button.
    for ev in click(5, 2) {
        app.handle_event(ev);
    }

    assert!(!app.dom().node(dlg).has_attribute("open"));
    assert_eq!(dialog::return_value(app.dom(), dlg), "confirm");
}

#[test]
fn form_method_dialog_submit_does_not_close_if_submit_handler_prevents() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let form = dom.create_element("form");
    dom.set_attribute(form, "method", "dialog").unwrap();
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(dlg, form).unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.dom_mut()
        .add_event_listener(form, "submit", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    dialog::show_modal(app.dom_mut(), dlg);
    app.draw_if_dirty().unwrap();

    for ev in click(1, 0) {
        app.handle_event(ev);
    }

    assert!(app.dom().node(dlg).has_attribute("open"));
}

#[test]
fn form_with_method_get_does_not_close_dialog_on_submit() {
    // Regression guard: only `method="dialog"` triggers the
    // auto-close. A normal form inside a dialog still fires
    // submit but leaves the dialog open.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let form = dom.create_element("form");
    // No method="dialog" — defaults to "get".
    let btn = dom.create_element("input");
    dom.set_attribute(btn, "type", "submit").unwrap();
    dom.append_child(form, btn).unwrap();
    dom.append_child(dlg, form).unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=submit]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    dialog::show_modal(app.dom_mut(), dlg);
    app.draw_if_dirty().unwrap();

    for ev in click(1, 0) {
        app.handle_event(ev);
    }

    assert!(app.dom().node(dlg).has_attribute("open"));
}

// ── Step 6: typed toggle event detail ──────────────────────────────

#[test]
fn dialog_show_and_close_fire_toggle_events_with_typed_state_transitions() {
    // Open + close transitions both fire a `toggle` event whose
    // detail.as_toggle() carries the correct old/new ToggleState.
    use rdom_core::ToggleState;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.append_child(root, dlg).unwrap();

    let mut app = test_app(dom, Stylesheet::new());

    let captured: Rc<RefCell<Vec<(ToggleState, ToggleState)>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(dlg, "toggle", ListenerOptions::default(), move |ctx| {
                let d = ctx
                    .event
                    .detail
                    .as_toggle()
                    .expect("toggle must carry EventDetail::Toggle");
                captured.borrow_mut().push((d.old_state, d.new_state));
            })
            .unwrap();
    }

    dialog::show(app.dom_mut(), dlg);
    dialog::close(app.dom_mut(), dlg, "");

    assert_eq!(
        *captured.borrow(),
        vec![
            (ToggleState::Closed, ToggleState::Open),
            (ToggleState::Open, ToggleState::Closed),
        ]
    );
}

#[test]
fn dialog_show_modal_fires_toggle_event_closed_to_open() {
    use rdom_core::ToggleState;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.append_child(root, dlg).unwrap();

    let mut app = test_app(dom, Stylesheet::new());
    let captured: Rc<Cell<Option<(ToggleState, ToggleState)>>> = Rc::new(Cell::new(None));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(dlg, "toggle", ListenerOptions::default(), move |ctx| {
                let d = ctx.event.detail.as_toggle().expect("typed Toggle detail");
                captured.set(Some((d.old_state, d.new_state)));
            })
            .unwrap();
    }

    dialog::show_modal(app.dom_mut(), dlg);

    assert_eq!(
        captured.get(),
        Some((ToggleState::Closed, ToggleState::Open))
    );
}

#[test]
fn dialog_show_on_already_open_does_not_refire_toggle() {
    // Calling show() / show_modal() on an already-open dialog is
    // idempotent — no state change, no toggle event.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.append_child(root, dlg).unwrap();
    let mut app = test_app(dom, Stylesheet::new());

    let count = Rc::new(Cell::new(0u32));
    {
        let count = count.clone();
        app.dom_mut()
            .add_event_listener(dlg, "toggle", ListenerOptions::default(), move |_| {
                count.set(count.get() + 1);
            })
            .unwrap();
    }

    dialog::show(app.dom_mut(), dlg);
    dialog::show(app.dom_mut(), dlg); // already open — no event
    dialog::show_modal(app.dom_mut(), dlg); // also no event (still open)
    assert_eq!(count.get(), 1);
}
