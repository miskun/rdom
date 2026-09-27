//! `:focus-visible` heuristics (Selectors 4 §13.2) through the App's
//! event loop: keyboard focus is evident, mouse focus on a control that
//! takes no keyboard input is not, text fields always are, a keypress
//! makes the current focus evident, script focus keeps the previous
//! element's visibility — and the UA focus indicators follow.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent,
    MouseEventKind,
};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{Overflow, Size};
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::cascade::computed_of;
use crate::style::{Color, Stylesheet, TuiStyle};

/// The UA control focus tint (`FOCUS-VOCAB-1`).
const TINT: Color = Color::Rgb(0x2d, 0x2f, 0x31);

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

fn press(app: &mut App<TestBackend>, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_event(CtEvent::Key(KeyEvent::new(code, modifiers)));
}

fn visible(app: &App<TestBackend>, id: NodeId) -> bool {
    app.dom().matches(id, ":focus-visible").unwrap()
}

fn focused(app: &App<TestBackend>, id: NodeId) -> bool {
    app.dom().matches(id, ":focus").unwrap()
}

/// One control per row under the UA sheet: rows 0 / 1 are buttons,
/// row 2 a text input, row 3 a checkbox. Returns (app, [a, b, text, cb]).
fn controls_app() -> (App<TestBackend>, [NodeId; 4]) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let row = |dom: &mut TuiDom, tag: &str, attrs: &[(&str, &str)], label: &str| {
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let el = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(el, k, v).unwrap();
        }
        if !label.is_empty() {
            let t = dom.create_text_node(label);
            dom.append_child(el, t).unwrap();
        }
        dom.append_child(div, el).unwrap();
        el
    };
    let a = row(&mut dom, "button", &[], "A");
    let b = row(&mut dom, "button", &[], "B");
    let text = row(&mut dom, "input", &[], "");
    let cb = row(&mut dom, "input", &[("type", "checkbox")], "");
    let terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, [a, b, text, cb])
}

#[test]
fn mouse_focus_on_a_button_matches_focus_but_not_focus_visible() {
    let (mut app, [a, _, _, cb]) = controls_app();
    click_at(&mut app, 1, 0);
    assert!(focused(&app, a), "the click focused the button");
    assert!(!visible(&app, a));
    click_at(&mut app, 1, 3);
    assert!(focused(&app, cb), "the click focused the checkbox");
    assert!(!visible(&app, cb));
}

#[test]
fn tab_focus_matches_focus_visible() {
    let (mut app, [a, b, _, _]) = controls_app();
    click_at(&mut app, 1, 0);
    assert!(!visible(&app, a));
    press(&mut app, KeyCode::Tab, KeyModifiers::empty());
    assert!(focused(&app, b));
    assert!(visible(&app, b), "keyboard focus is evident");
}

#[test]
fn mouse_focus_in_a_text_input_matches_focus_visible() {
    let (mut app, [_, _, text, _]) = controls_app();
    click_at(&mut app, 1, 2);
    assert!(focused(&app, text));
    assert!(visible(&app, text), "text fields always show focus");
}

/// Browsers switch `:focus-visible` on when the user presses a key
/// while a mouse-focused element keeps focus; a key chord with Ctrl /
/// Alt / Super (a shortcut, not typing or navigation) does not.
#[test]
fn a_keypress_after_mouse_focus_makes_it_visible() {
    let (mut app, [a, _, _, _]) = controls_app();
    click_at(&mut app, 1, 0);
    press(&mut app, KeyCode::Char('x'), KeyModifiers::ALT);
    assert!(!visible(&app, a), "a shortcut chord is not keyboard use");
    press(&mut app, KeyCode::Char('x'), KeyModifiers::empty());
    assert!(focused(&app, a));
    assert!(visible(&app, a));
}

/// Selectors 4 §13.2: script focus is evident iff the previously
/// focused element's focus was.
#[test]
fn script_focus_keeps_the_previous_visibility() {
    let (mut app, [a, b, _, _]) = controls_app();
    click_at(&mut app, 1, 0);
    crate::runtime::focus::focus_node(app.dom_mut(), Some(b));
    assert!(!visible(&app, b), "after mouse focus");
    press(&mut app, KeyCode::Tab, KeyModifiers::SHIFT);
    assert!(visible(&app, a));
    crate::runtime::focus::focus_node(app.dom_mut(), Some(b));
    assert!(visible(&app, b), "after keyboard focus");
}

/// The UA control tint keys on `:focus-visible`: absent after a mouse
/// click on a button, present on a clicked text input, and restyled on
/// the next frame when a keypress turns it on without moving focus.
#[test]
fn the_ua_focus_tint_follows_focus_visible() {
    let (mut app, [a, _, text, _]) = controls_app();
    click_at(&mut app, 1, 0);
    app.draw_if_dirty().unwrap();
    assert_ne!(computed_of(app.dom(), a).bg, TINT, "no tint on mouse focus");

    press(&mut app, KeyCode::Char('x'), KeyModifiers::empty());
    app.draw_if_dirty().unwrap();
    assert_eq!(computed_of(app.dom(), a).bg, TINT, "the keypress restyles");

    click_at(&mut app, 1, 2);
    app.draw_if_dirty().unwrap();
    assert_eq!(computed_of(app.dom(), text).bg, TINT);
}

/// The scroll-focus thumb is a focus indicator too: a mouse-focused
/// element inside an overflowing scroller does not light it; the next
/// keypress does.
#[test]
fn the_scroll_focus_marker_waits_for_visible_focus() {
    use crate::runtime::scrollbar::SCROLL_FOCUS_ATTR;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = dom.create_element("pane");
    dom.append_child(root, pane).unwrap();
    let f = dom.create_element("f");
    dom.set_attribute(f, "tabindex", "0").unwrap();
    let t = dom.create_text_node("focus me");
    dom.append_child(f, t).unwrap();
    dom.append_child(pane, f).unwrap();
    for _ in 0..5 {
        let r = dom.create_element("r");
        dom.append_child(pane, r).unwrap();
    }
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "pane",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked("f", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("r", TuiStyle::new().height(Size::Fixed(1)));
    let terminal = Terminal::new(TestBackend::new(12, 5)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();

    click_at(&mut app, 1, 0);
    assert!(focused(&app, f));
    app.draw_if_dirty().unwrap();
    assert!(!app.dom().node(pane).has_attribute(SCROLL_FOCUS_ATTR));

    press(&mut app, KeyCode::Char('x'), KeyModifiers::empty());
    app.draw_if_dirty().unwrap();
    assert!(app.dom().node(pane).has_attribute(SCROLL_FOCUS_ATTR));
}
