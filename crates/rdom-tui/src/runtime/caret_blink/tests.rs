//! Caret blink (`P7-CARET-BLINK-1`) through a headless `App`: the
//! virtual clock moves only by `App::advance`, and the screen is the
//! `TestBackend`'s bytes replayed through `VirtualScreen`.

use std::time::Duration;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{Color, Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const PERIOD: u64 = 530;

/// `<p contenteditable>hello</p><button>b</button>`, caret after "he".
fn app(blink: Option<Duration>) -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let button = dom.create_element("button");
    let bt = dom.create_text_node("b");
    dom.append_child(button, bt).unwrap();
    dom.append_child(root, button).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "p, button",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(20)),
    );
    let terminal = Terminal::new(TestBackend::new(30, 4)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal)
        .unwrap()
        .with_caret_blink(blink);
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));
    app.advance(0).unwrap();
    (app, p, t, button)
}

fn blinking() -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    app(Some(Duration::from_millis(PERIOD)))
}

/// Whether the caret cell at `(x, 0)` shows the caret (the unstyled
/// caret paints a white background).
fn caret_shown_at(app: &App<TestBackend>, x: u16) -> bool {
    let mut screen = VirtualScreen::new(30, 4);
    screen.apply(app.terminal().backend().bytes());
    screen.cell(x, 0).unwrap().bg != Color::Reset
}

fn press(app: &mut App<TestBackend>, code: KeyCode) {
    app.handle_event(CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
    app.advance(0).unwrap();
}

#[test]
fn the_caret_is_visible_once_an_editable_is_focused() {
    let (app, ..) = blinking();
    assert!(caret_shown_at(&app, 2));
}

#[test]
fn the_caret_hides_after_one_period_and_returns_after_two() {
    let (mut app, ..) = blinking();
    app.advance(PERIOD - 1).unwrap();
    assert!(caret_shown_at(&app, 2), "still on just before the period");
    app.advance(1).unwrap();
    assert!(!caret_shown_at(&app, 2), "off after one period");
    app.advance(PERIOD).unwrap();
    assert!(caret_shown_at(&app, 2), "on again after two");
}

#[test]
fn an_edit_shows_the_caret_and_restarts_the_period() {
    let (mut app, _p, t, _b) = blinking();
    app.advance(PERIOD).unwrap();
    assert!(!caret_shown_at(&app, 2));
    press(&mut app, KeyCode::Char('X'));
    assert_eq!(app.dom().node(t).node_value(), Some("heXllo"));
    assert!(caret_shown_at(&app, 3), "visible right after the edit");
    app.advance(PERIOD - 1).unwrap();
    assert!(caret_shown_at(&app, 3), "the period restarts at the edit");
    app.advance(1).unwrap();
    assert!(!caret_shown_at(&app, 3));
}

#[test]
fn a_caret_move_shows_the_caret() {
    let (mut app, ..) = blinking();
    app.advance(PERIOD).unwrap();
    press(&mut app, KeyCode::Left);
    assert!(caret_shown_at(&app, 1));
}

#[test]
fn a_delete_that_keeps_the_caret_in_place_still_shows_it() {
    let (mut app, ..) = blinking();
    app.advance(PERIOD).unwrap();
    press(&mut app, KeyCode::Delete);
    assert!(caret_shown_at(&app, 2));
}

#[test]
fn no_wakeup_is_scheduled_without_a_focused_editable() {
    let (mut app, _p, _t, button) = blinking();
    assert!(
        app.caret_blink_deadline().is_some(),
        "a focused editable blinks"
    );
    app.dom_mut().set_focused(Some(button));
    app.advance(0).unwrap();
    assert_eq!(app.caret_blink_deadline(), None);
    app.dom_mut().set_focused(None);
    app.advance(PERIOD * 3).unwrap();
    assert_eq!(app.caret_blink_deadline(), None);
}

#[test]
fn a_range_selection_does_not_blink() {
    // No caret is painted for a range, so there is nothing to blink.
    let (mut app, _p, t, _b) = blinking();
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 3),
    )));
    app.advance(0).unwrap();
    assert_eq!(app.caret_blink_deadline(), None);
}

#[test]
fn a_blink_off_phase_does_not_leak_into_the_next_focus() {
    let (mut app, p, t, button) = blinking();
    app.advance(PERIOD).unwrap();
    assert!(!caret_shown_at(&app, 2));
    app.dom_mut().set_focused(Some(button));
    app.advance(0).unwrap();
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));
    app.advance(0).unwrap();
    assert!(caret_shown_at(&app, 2), "refocus starts visible");
}

#[test]
fn an_unfocused_terminal_hides_the_caret_and_stops_blinking() {
    // Browsers paint no caret in an inactive window.
    let (mut app, ..) = blinking();
    app.handle_event(CtEvent::FocusLost);
    app.advance(0).unwrap();
    assert!(!caret_shown_at(&app, 2));
    assert_eq!(app.caret_blink_deadline(), None);
    app.handle_event(CtEvent::FocusGained);
    app.advance(0).unwrap();
    assert!(caret_shown_at(&app, 2));
    assert!(app.caret_blink_deadline().is_some());
}

#[test]
fn blink_is_off_unless_enabled_on_a_custom_backend() {
    let (mut app, ..) = app(None);
    assert_eq!(app.caret_blink_deadline(), None);
    app.advance(PERIOD * 3).unwrap();
    assert!(caret_shown_at(&app, 2), "a steady caret");
}
