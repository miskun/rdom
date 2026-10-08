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

/// A click on the middle of `id`'s laid-out box.
fn click_on(app: &App<TestBackend>, id: rdom_core::NodeId) -> Vec<CtEvent> {
    use crate::node::TuiNodeExt;
    let r = app.dom().node(id).layout_rect().expect("laid out");
    click(
        (r.x + i32::from(r.width) / 2) as u16,
        (r.y + i32::from(r.height) / 2) as u16,
    )
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
    dialog::show(app.dom_mut(), dlg).unwrap();
    assert!(app.dom().node(dlg).has_attribute("open"));
    assert!(!dialog::is_modal(app.dom(), dlg));
}

#[test]
fn show_modal_sets_open_and_modal_marker() {
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    assert!(app.dom().node(dlg).has_attribute("open"));
    assert!(dialog::is_modal(app.dom(), dlg));
}

#[test]
fn close_clears_open_attribute_and_stores_return_value() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg).unwrap();
    dialog::close(app.dom_mut(), dlg, "ok");
    assert!(!app.dom().node(dlg).has_attribute("open"));
    assert_eq!(dialog::return_value(app.dom(), dlg), "ok");
}

#[test]
fn close_fires_close_event_on_dialog() {
    let (mut app, dlg) = dialog_app();
    dialog::show(app.dom_mut(), dlg).unwrap();
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
    dialog::show(app.dom_mut(), dlg).unwrap();
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
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
    dialog::show(app.dom_mut(), dlg).unwrap();
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();

    // The modal dialog is centred in the viewport (UA `dialog:modal`).
    for ev in click_on(&app, btn) {
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();

    for ev in click_on(&app, btn) {
        app.handle_event(ev);
    }

    assert!(app.dom().node(dlg).has_attribute("open"));
}

/// P7-FORM-OWNER-1 (HTML §4.10.19.6): the submitter's `formmethod`
/// overrides the form's `method` — `formmethod="dialog"` closes the
/// dialog from a `method="get"` form, `formmethod="get"` keeps it open
/// from a `method="dialog"` form.
#[test]
fn submitter_formmethod_overrides_the_forms_method_for_dialog_close() {
    use crate::accessors::TuiAccessorsMut;
    for (form_method, button_method, closes) in [("get", "dialog", true), ("dialog", "get", false)]
    {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let dlg = dom.create_element("dialog");
        let form = dom.create_element("form");
        dom.set_attribute(form, "method", form_method).unwrap();
        let btn = dom.create_element("button");
        dom.set_attribute(btn, "formmethod", button_method).unwrap();
        dom.set_attribute(btn, "value", "ok").unwrap();
        dom.append_child(form, btn).unwrap();
        dom.append_child(dlg, form).unwrap();
        dom.append_child(root, dlg).unwrap();
        let mut app = test_app(dom, Stylesheet::new());
        dialog::show_modal(app.dom_mut(), dlg).unwrap();
        app.draw_if_dirty().unwrap();

        app.dom_mut().node_mut(btn).click();

        assert_eq!(
            !app.dom().node(dlg).has_attribute("open"),
            closes,
            "method={form_method} formmethod={button_method}"
        );
        if closes {
            assert_eq!(dialog::return_value(app.dom(), dlg), "ok");
        }
    }
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
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();

    for ev in click_on(&app, btn) {
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

    dialog::show(app.dom_mut(), dlg).unwrap();
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

    dialog::show_modal(app.dom_mut(), dlg).unwrap();

    assert_eq!(
        captured.get(),
        Some((ToggleState::Closed, ToggleState::Open))
    );
}

#[test]
fn dialog_show_on_already_open_does_not_refire_toggle() {
    // Calling show() on an already-open dialog is idempotent, and
    // show_modal() on one is refused — no state change, no toggle event.
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

    dialog::show(app.dom_mut(), dlg).unwrap();
    dialog::show(app.dom_mut(), dlg).unwrap(); // already open — no event
    // HTML §4.11.4 step 2: showModal() on an open dialog throws, no event.
    assert!(dialog::show_modal(app.dom_mut(), dlg).is_err());
    assert_eq!(count.get(), 1);
}

// ── HTML §4.11.4 modal dialogs: focusing steps, focus trap, Esc ─────

/// Dialog with two buttons inside and one button outside.
fn modal_fixture() -> (App<TestBackend>, NodeId, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outside = dom.create_element("button");
    let dlg = dom.create_element("dialog");
    let first = dom.create_element("button");
    let second = dom.create_element("button");
    dom.append_child(dlg, first).unwrap();
    dom.append_child(dlg, second).unwrap();
    dom.append_child(root, outside).unwrap();
    dom.append_child(root, dlg).unwrap();
    let app = test_app(dom, Stylesheet::new());
    (app, dlg, outside, first, second)
}

/// `showModal()` runs the dialog focusing steps: the first focusable
/// descendant gets focus, and the previously focused element is
/// remembered so `close()` can return focus to it.
#[test]
fn show_modal_focuses_first_descendant_and_close_restores_focus() {
    let (mut app, dlg, outside, first, _) = modal_fixture();
    app.draw_if_dirty().unwrap();
    app.dom_mut().set_focused(Some(outside));
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(
        app.dom().focused(),
        Some(first),
        "focus moves into the dialog"
    );
    dialog::close(app.dom_mut(), dlg, "");
    assert_eq!(
        app.dom().focused(),
        Some(outside),
        "focus returns to the previously focused element"
    );
}

/// While a modal dialog is open the rest of the document is inert:
/// Tab cycles among the dialog's focusable descendants only.
#[test]
fn tab_is_trapped_inside_an_open_modal_dialog() {
    let (mut app, dlg, outside, first, second) = modal_fixture();
    app.draw_if_dirty().unwrap();
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(first));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(second));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(first),
        "wraps inside the dialog, never reaches `outside`"
    );
    let _ = outside;
}

/// Esc cancels the open modal dialog even when focus is not inside it
/// (with the rest of the document inert, the dialog is the only
/// interactive content).
#[test]
fn esc_cancels_the_open_modal_regardless_of_focus_location() {
    let (mut app, dlg, outside, _, _) = modal_fixture();
    app.draw_if_dirty().unwrap();
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.draw_if_dirty().unwrap();
    // Force focus elsewhere (a consumer calling focus() directly).
    app.dom_mut().set_focused(Some(outside));
    app.handle_event(key(KeyCode::Esc));
    assert!(
        !app.dom().node(dlg).has_attribute("open"),
        "modal closed by Esc"
    );
}

/// `close()` returns focus only to an element that is still focusable;
/// a previously focused element that became disabled is skipped (HTML
/// falls back to the body — here, no focus).
#[test]
fn close_does_not_focus_a_previously_focused_element_that_is_no_longer_focusable() {
    let (mut app, dlg, outside, _, _) = modal_fixture();
    app.draw_if_dirty().unwrap();
    app.dom_mut().set_focused(Some(outside));
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    app.dom_mut()
        .set_attribute(outside, "disabled", "")
        .unwrap();
    dialog::close(app.dom_mut(), dlg, "");
    assert_ne!(app.dom().focused(), Some(outside));
}

// ── HTML §4.11.4's guards ──────────────────────────────────────────

/// "Show a modal dialog" step 1: `showModal()` on a dialog that is
/// already modal returns — it is not moved to the top of the top layer,
/// so a modal dialog above it keeps the page (and it) inert.
#[test]
fn show_modal_on_a_modal_dialog_does_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("dialog");
    let b = dom.create_element("dialog");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dialog::show_modal(&mut dom, a).unwrap();
    dialog::show_modal(&mut dom, b).unwrap();
    dialog::show_modal(&mut dom, a).unwrap();
    assert_eq!(dom.top_layer(), [a, b]);
    assert_eq!(dialog::top_modal(&dom), Some(b));
}

/// "Show a modal dialog" steps 2, 4 and 5: an open non-modal dialog, a
/// disconnected one and one showing as a popover throw
/// `InvalidStateError` — and stay as they were.
#[test]
fn show_modal_throws_on_an_open_disconnected_or_popover_dialog() {
    use rdom_core::{DomError, TopLayerKind};
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let open = dom.create_element("dialog");
    dom.append_child(root, open).unwrap();
    dialog::show(&mut dom, open).unwrap();
    assert!(matches!(
        dialog::show_modal(&mut dom, open),
        Err(DomError::InvalidState(_))
    ));
    assert!(!dialog::is_modal(&dom, open));

    let loose = dom.create_element("dialog");
    assert!(matches!(
        dialog::show_modal(&mut dom, loose),
        Err(DomError::InvalidState(_))
    ));
    assert!(!dom.node(loose).has_attribute("open"));

    let pop = dom.create_element("dialog");
    dom.set_attribute(pop, "popover", "manual").unwrap();
    dom.append_child(root, pop).unwrap();
    crate::runtime::builtins::popover::show_popover(&mut dom, pop).unwrap();
    assert!(matches!(
        dialog::show_modal(&mut dom, pop),
        Err(DomError::InvalidState(_))
    ));
    assert_eq!(dom.top_layer_kind(pop), Some(TopLayerKind::Popover));
}

/// `show()` steps 1–2: on an open non-modal dialog it returns; on a modal
/// one it throws `InvalidStateError` (it no longer turns a modal dialog
/// non-modal).
#[test]
fn show_on_a_modal_dialog_throws() {
    use rdom_core::DomError;
    let (mut app, dlg) = dialog_app();
    dialog::show_modal(app.dom_mut(), dlg).unwrap();
    assert!(matches!(
        dialog::show(app.dom_mut(), dlg),
        Err(DomError::InvalidState(_))
    ));
    assert!(dialog::is_modal(app.dom(), dlg));
}

// ── HTML §4.11.4 `beforetoggle` (Living Standard, 2026-10) ─────────

/// One dialog in a bare document, with a log of its `beforetoggle` /
/// `toggle` events: type, old → new, cancelable, and whether the dialog
/// had `open` when the event was dispatched.
type ToggleLog = Rc<RefCell<Vec<(String, &'static str, bool, bool)>>>;

fn logged_dialog() -> (TuiDom, NodeId, ToggleLog) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.append_child(root, dlg).unwrap();
    let log: ToggleLog = Rc::new(RefCell::new(Vec::new()));
    for ty in ["beforetoggle", "toggle"] {
        let log = log.clone();
        dom.add_event_listener(dlg, ty, ListenerOptions::default(), move |ctx| {
            let d = ctx.event.detail.as_toggle().expect("typed Toggle detail");
            let change = match d.new_state {
                rdom_core::ToggleState::Open => "closed->open",
                rdom_core::ToggleState::Closed => "open->closed",
            };
            let open = ctx.dom.node(dlg).has_attribute("open");
            log.borrow_mut().push((
                ctx.event.event_type.clone(),
                change,
                ctx.event.cancelable,
                open,
            ));
        })
        .unwrap();
    }
    (dom, dlg, log)
}

/// `show()` step 3, `showModal()` step 6, "close the dialog" step 2: a
/// `beforetoggle` fires before the change — cancelable `closed` → `open`
/// before `open` is added, non-cancelable `open` → `closed` before it is
/// removed — and then the `toggle`.
#[test]
fn show_show_modal_and_close_fire_beforetoggle_first() {
    for modal in [false, true] {
        let (mut dom, dlg, log) = logged_dialog();
        if modal {
            dialog::show_modal(&mut dom, dlg).unwrap();
        } else {
            dialog::show(&mut dom, dlg).unwrap();
        }
        dialog::close(&mut dom, dlg, "");
        let got: Vec<_> = log
            .borrow()
            .iter()
            .map(|(t, c, can, open)| (t.as_str().to_string(), *c, *can, *open))
            .collect();
        assert_eq!(
            got,
            [
                ("beforetoggle".to_string(), "closed->open", true, false),
                ("toggle".to_string(), "closed->open", false, true),
                ("beforetoggle".to_string(), "open->closed", false, true),
                ("toggle".to_string(), "open->closed", false, false),
            ],
            "modal: {modal}"
        );
    }
}

/// A canceled `beforetoggle` stops the show: no `open`, no top layer, no
/// `toggle`, no focus change — and it is not an error (the methods
/// return).
#[test]
fn a_canceled_beforetoggle_keeps_the_dialog_closed() {
    for modal in [false, true] {
        let (mut dom, dlg, log) = logged_dialog();
        let root = dom.root();
        let inner = dom.create_element("button");
        dom.append_child(dlg, inner).unwrap();
        let outside = dom.create_element("button");
        dom.append_child(root, outside).unwrap();
        crate::runtime::focus::focus_node(&mut dom, Some(outside));
        dom.add_event_listener(dlg, "beforetoggle", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
        let shown = if modal {
            dialog::show_modal(&mut dom, dlg)
        } else {
            dialog::show(&mut dom, dlg)
        };
        assert!(shown.is_ok(), "modal: {modal}");
        assert!(!dom.node(dlg).has_attribute("open"), "modal: {modal}");
        assert_eq!(dom.top_layer_kind(dlg), None, "modal: {modal}");
        assert_eq!(dom.focused(), Some(outside), "modal: {modal}");
        let types: Vec<String> = log.borrow().iter().map(|e| e.0.clone()).collect();
        assert_eq!(types, ["beforetoggle"], "modal: {modal}");
    }
}

/// `showModal()` steps 7–9: a `beforetoggle` listener that opens the
/// dialog itself, or disconnects it, ends the show without an error and
/// without a second `toggle`.
#[test]
fn show_modal_rechecks_after_beforetoggle() {
    let (mut dom, dlg, log) = logged_dialog();
    dom.add_event_listener(
        dlg,
        "beforetoggle",
        ListenerOptions::default().with_once(true),
        move |ctx| {
            dialog::show(ctx.dom, dlg).unwrap();
        },
    )
    .unwrap();
    dialog::show_modal(&mut dom, dlg).unwrap();
    assert!(dom.node(dlg).has_attribute("open"));
    assert!(!dialog::is_modal(&dom, dlg), "the listener's show() won");
    let toggles = log.borrow().iter().filter(|e| e.0 == "toggle").count();
    assert_eq!(toggles, 1);

    let (mut dom, dlg, log) = logged_dialog();
    let root = dom.root();
    dom.add_event_listener(
        dlg,
        "beforetoggle",
        ListenerOptions::default(),
        move |ctx| {
            ctx.dom.remove_child(root, dlg).unwrap();
        },
    )
    .unwrap();
    dialog::show_modal(&mut dom, dlg).unwrap();
    assert!(!dom.node(dlg).has_attribute("open"));
    assert_eq!(dom.top_layer_kind(dlg), None);
    assert!(log.borrow().iter().all(|e| e.0 != "toggle"));
}

/// "Close the dialog" step 3: a `beforetoggle` listener that closes the
/// dialog first ends the outer close — one `close` event, one `toggle`.
#[test]
fn close_rechecks_after_beforetoggle() {
    let (mut dom, dlg, log) = logged_dialog();
    dialog::show(&mut dom, dlg).unwrap();
    let closes = Rc::new(Cell::new(0u32));
    {
        let closes = closes.clone();
        dom.add_event_listener(dlg, "close", ListenerOptions::default(), move |_| {
            closes.set(closes.get() + 1);
        })
        .unwrap();
    }
    dom.add_event_listener(
        dlg,
        "beforetoggle",
        ListenerOptions::default().with_once(true),
        move |ctx| {
            dialog::close(ctx.dom, dlg, "inner");
        },
    )
    .unwrap();
    dialog::close(&mut dom, dlg, "outer");
    assert!(!dom.node(dlg).has_attribute("open"));
    assert_eq!(closes.get(), 1);
    assert_eq!(dialog::return_value(&dom, dlg), "inner");
    let toggles = log.borrow().iter().filter(|e| e.0 == "toggle").count();
    assert_eq!(toggles, 2, "one for the show, one for the close");
}
