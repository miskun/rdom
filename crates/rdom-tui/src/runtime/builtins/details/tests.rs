//! `<details>` + `<summary>` toggle tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

fn test_app(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

/// <details><summary>Title</summary><p>body</p></details>
fn details_fixture() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let details = dom.create_element("details");
    let summary = dom.create_element("summary");
    let t = dom.create_text_node("Title");
    dom.append_child(summary, t).unwrap();
    dom.append_child(details, summary).unwrap();
    let body = dom.create_element("p");
    let bt = dom.create_text_node("body");
    dom.append_child(body, bt).unwrap();
    dom.append_child(details, body).unwrap();
    dom.append_child(root, details).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "summary",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let app = test_app(dom, sheet);
    (app, details, summary)
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

// ── Click toggles ──────────────────────────────────────────────────

#[test]
fn click_on_summary_toggles_open_attribute() {
    let (mut app, details, _) = details_fixture();
    app.draw_if_dirty().unwrap();
    assert!(!app.dom().node(details).has_attribute("open"));

    click_at(&mut app, 1, 0);
    assert!(app.dom().node(details).has_attribute("open"));

    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(details).has_attribute("open"));
}

#[test]
fn click_on_summary_fires_toggle_event_on_details() {
    let (mut app, details, _) = details_fixture();
    let fires = Rc::new(Cell::new(0u32));
    let f = fires.clone();
    app.dom_mut()
        .add_event_listener(details, "toggle", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert_eq!(fires.get(), 1);
    click_at(&mut app, 1, 0);
    assert_eq!(fires.get(), 2);
}

#[test]
fn click_on_text_inside_summary_still_toggles() {
    let (mut app, details, _) = details_fixture();
    app.draw_if_dirty().unwrap();
    // Click on cell (2, 0) — inside the "Title" text.
    click_at(&mut app, 2, 0);
    assert!(app.dom().node(details).has_attribute("open"));
}

#[test]
fn click_on_summary_not_child_of_details_does_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    // Orphan summary directly under root.
    let summary = dom.create_element("summary");
    let t = dom.create_text_node("Orphan");
    dom.append_child(summary, t).unwrap();
    dom.append_child(root, summary).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "summary",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    // Nothing to toggle — just not crashing is the test.
    assert!(!app.dom().node(summary).has_attribute("open"));
}

// ── Keyboard toggles ───────────────────────────────────────────────

#[test]
fn enter_on_focused_summary_toggles() {
    let (mut app, details, summary) = details_fixture();
    app.dom_mut().set_focused(Some(summary));
    app.handle_event(key_press(KeyCode::Enter, KeyModifiers::empty()));
    assert!(app.dom().node(details).has_attribute("open"));
}

#[test]
fn space_on_focused_summary_toggles() {
    let (mut app, details, summary) = details_fixture();
    app.dom_mut().set_focused(Some(summary));
    app.handle_event(key_press(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.dom().node(details).has_attribute("open"));
}

#[test]
fn ctrl_enter_does_not_toggle() {
    let (mut app, details, summary) = details_fixture();
    app.dom_mut().set_focused(Some(summary));
    app.handle_event(key_press(KeyCode::Enter, KeyModifiers::CONTROL));
    assert!(!app.dom().node(details).has_attribute("open"));
}

// ── preventDefault ─────────────────────────────────────────────────

#[test]
fn prevent_default_on_click_blocks_toggle() {
    let (mut app, details, summary) = details_fixture();
    app.dom_mut()
        .add_event_listener(summary, "click", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(details).has_attribute("open"));
}

// ── Initial state respected ────────────────────────────────────────

#[test]
fn initially_open_details_toggles_closed_on_click() {
    let (mut app, details, _) = details_fixture();
    // Set open at init.
    app.dom_mut().set_attribute(details, "open", "").unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!app.dom().node(details).has_attribute("open"));
}

// ── Toggle event does not bubble ───────────────────────────────────

#[test]
fn toggle_event_does_not_bubble_to_root() {
    let (mut app, _, _) = details_fixture();
    let root = app.dom().root();
    let saw = Rc::new(Cell::new(false));
    let s = saw.clone();
    app.dom_mut()
        .add_event_listener(root, "toggle", ListenerOptions::default(), move |_| {
            s.set(true);
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert!(!saw.get(), "toggle should not bubble past details");
}

// ── Step 6: typed toggle event detail ──────────────────────────────

#[test]
fn details_toggle_event_carries_typed_state_transitions() {
    // Canonical step-6 failing test: clicking the summary fires a
    // toggle event whose detail.as_toggle() reads Closed → Open on
    // the first click and Open → Closed on the second.
    use rdom_core::ToggleState;
    let (mut app, details, _) = details_fixture();

    let captured: Rc<RefCell<Vec<(ToggleState, ToggleState)>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(details, "toggle", ListenerOptions::default(), move |ctx| {
                let d = ctx
                    .event
                    .detail
                    .as_toggle()
                    .expect("toggle must carry EventDetail::Toggle");
                captured.borrow_mut().push((d.old_state, d.new_state));
            })
            .unwrap();
    }
    app.draw_if_dirty().unwrap();

    click_at(&mut app, 1, 0);
    click_at(&mut app, 1, 0);

    assert_eq!(
        *captured.borrow(),
        vec![
            (ToggleState::Closed, ToggleState::Open),
            (ToggleState::Open, ToggleState::Closed),
        ]
    );
}
