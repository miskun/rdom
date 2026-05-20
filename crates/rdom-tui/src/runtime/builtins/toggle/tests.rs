//! `<input type="checkbox">` + `<input type="radio">` default
//! action tests.

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
use crate::style::{Stylesheet, TuiStyle};

fn test_app(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

fn click_at(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
}

fn key_press(code: KeyCode, modifiers: KeyModifiers) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// Build a `<input type="checkbox">` parented under root, with a UA
/// stylesheet overridden to give the widget a fixed (10×1) size for
/// predictable click coordinates.
fn checkbox_app() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.append_child(root, cb).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=checkbox]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let app = test_app(dom, sheet);
    (app, cb)
}

/// Build a radio group: three radios with `name="g"`. Returns
/// (app, [r1, r2, r3]).
fn radio_group_app() -> (App<TestBackend>, [NodeId; 3]) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let make = |dom: &mut TuiDom, i: usize| {
        let r = dom.create_element("input");
        dom.set_attribute(r, "type", "radio").unwrap();
        dom.set_attribute(r, "name", "g").unwrap();
        dom.set_attribute(r, "id", &format!("r{}", i)).unwrap();
        dom.append_child(root, r).unwrap();
        r
    };
    let ids = [make(&mut dom, 0), make(&mut dom, 1), make(&mut dom, 2)];
    let sheet = Stylesheet::new().rule_unchecked(
        "input[type=radio]",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let app = test_app(dom, sheet);
    (app, ids)
}

// ── Checkbox: click toggles ───────────────────────────────────────

#[test]
fn click_on_checkbox_toggles_checked_attribute() {
    let (mut app, cb) = checkbox_app();
    app.draw_if_dirty().unwrap();
    assert!(!app.dom().node(cb).has_attribute("checked"));

    click_at(&mut app, 1, 0);
    assert!(app.dom().node(cb).has_attribute("checked"));

    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

#[test]
fn click_on_checkbox_fires_input_then_change() {
    let (mut app, cb) = checkbox_app();
    let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    for ty in ["input", "change"] {
        let o = order.clone();
        app.dom_mut()
            .add_event_listener(cb, ty, ListenerOptions::default(), move |_| {
                o.borrow_mut().push(ty);
            })
            .unwrap();
    }
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);

    assert_eq!(*order.borrow(), vec!["input", "change"]);
}

#[test]
fn prevent_default_on_click_blocks_checkbox_toggle() {
    let (mut app, cb) = checkbox_app();
    app.dom_mut()
        .add_event_listener(cb, "click", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

#[test]
fn disabled_checkbox_does_not_toggle_on_click() {
    let (mut app, cb) = checkbox_app();
    app.dom_mut().set_attribute(cb, "disabled", "").unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

#[test]
fn initially_checked_checkbox_unchecks_on_click() {
    let (mut app, cb) = checkbox_app();
    app.dom_mut().set_attribute(cb, "checked", "").unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

// ── Checkbox: Space activates ─────────────────────────────────────

#[test]
fn space_on_focused_checkbox_toggles() {
    let (mut app, cb) = checkbox_app();
    app.dom_mut().set_focused(Some(cb));
    app.handle_event(key_press(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.dom().node(cb).has_attribute("checked"));
}

#[test]
fn enter_on_focused_checkbox_does_not_toggle() {
    // Per HTML, Enter on a checkbox does NOT toggle (it submits the
    // surrounding form, when one exists). Enter activation is
    // button-only. C.4c will wire up the form-submit redirect.
    let (mut app, cb) = checkbox_app();
    app.dom_mut().set_focused(Some(cb));
    app.handle_event(key_press(KeyCode::Enter, KeyModifiers::empty()));
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

#[test]
fn ctrl_space_does_not_toggle_checkbox() {
    let (mut app, cb) = checkbox_app();
    app.dom_mut().set_focused(Some(cb));
    app.handle_event(key_press(KeyCode::Char(' '), KeyModifiers::CONTROL));
    assert!(!app.dom().node(cb).has_attribute("checked"));
}

// ── Radio: click selects + sibling unchecking ─────────────────────

#[test]
fn click_on_radio_checks_it_and_unchecks_siblings() {
    // Drive r2's selection via the focused-Space activation path
    // — equivalent to a click on r2, but layout-independent so the
    // test isn't sensitive to inline / block flow nuances.
    let (mut app, [r1, r2, r3]) = radio_group_app();
    app.dom_mut().set_attribute(r1, "checked", "").unwrap();
    app.dom_mut().set_focused(Some(r2));
    app.handle_event(key_press(KeyCode::Char(' '), KeyModifiers::empty()));

    assert!(!app.dom().node(r1).has_attribute("checked"));
    assert!(app.dom().node(r2).has_attribute("checked"));
    assert!(!app.dom().node(r3).has_attribute("checked"));
}

#[test]
fn re_clicking_already_checked_radio_is_noop() {
    let (mut app, [r1, _r2, _r3]) = radio_group_app();
    app.dom_mut().set_attribute(r1, "checked", "").unwrap();
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(r1, "change", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();

    click_at(&mut app, 1, 0);

    assert!(app.dom().node(r1).has_attribute("checked"));
    assert_eq!(fired.get(), 0, "no change event when state didn't change");
}

#[test]
fn radio_groups_with_different_names_are_independent() {
    let (mut app, [r1, _, _]) = radio_group_app();
    let r_other = app.dom_mut().create_element("input");
    app.dom_mut()
        .set_attribute(r_other, "type", "radio")
        .unwrap();
    app.dom_mut()
        .set_attribute(r_other, "name", "other")
        .unwrap();
    app.dom_mut().set_attribute(r_other, "checked", "").unwrap();
    let root = app.dom().root();
    app.dom_mut().append_child(root, r_other).unwrap();
    app.draw_if_dirty().unwrap();

    // Click r1 in group "g" — must not touch r_other in group "other".
    click_at(&mut app, 1, 0);
    assert!(app.dom().node(r1).has_attribute("checked"));
    assert!(app.dom().node(r_other).has_attribute("checked"));
}

// ── Radio: arrow-key navigation ────────────────────────────────────

#[test]
fn down_arrow_moves_focus_to_next_radio_in_group() {
    let (mut app, [r1, r2, _r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r1));
    app.handle_event(key_press(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r2));
}

#[test]
fn right_arrow_also_moves_to_next_radio() {
    let (mut app, [r1, r2, _r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r1));
    app.handle_event(key_press(KeyCode::Right, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r2));
}

#[test]
fn up_arrow_moves_focus_to_previous_radio_in_group() {
    let (mut app, [r1, r2, _r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r2));
    app.handle_event(key_press(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r1));
}

#[test]
fn down_arrow_at_last_radio_wraps_to_first() {
    let (mut app, [r1, _r2, r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r3));
    app.handle_event(key_press(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r1));
}

#[test]
fn up_arrow_at_first_radio_wraps_to_last() {
    let (mut app, [r1, _r2, r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r1));
    app.handle_event(key_press(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r3));
}

#[test]
fn arrow_with_modifier_does_not_navigate() {
    let (mut app, [r1, _, _]) = radio_group_app();
    app.dom_mut().set_focused(Some(r1));
    app.handle_event(key_press(KeyCode::Down, KeyModifiers::SHIFT));
    assert_eq!(app.dom().focused(), Some(r1));
}

#[test]
fn arrow_on_radio_without_name_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let r = dom.create_element("input");
    dom.set_attribute(r, "type", "radio").unwrap();
    // No `name` — not in any group.
    dom.append_child(root, r).unwrap();
    let mut app = test_app(dom, Stylesheet::new());
    app.dom_mut().set_focused(Some(r));
    app.handle_event(key_press(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.dom().focused(), Some(r));
}

// ── Space activation on radio ──────────────────────────────────────

#[test]
fn space_on_focused_radio_selects_it() {
    let (mut app, [_r1, r2, _r3]) = radio_group_app();
    app.dom_mut().set_focused(Some(r2));
    app.handle_event(key_press(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.dom().node(r2).has_attribute("checked"));
}
