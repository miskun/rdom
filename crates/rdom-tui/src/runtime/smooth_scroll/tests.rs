//! Smooth scrolling (`P7-SCROLL-BEHAVIOR-1`) through a headless `App`:
//! the virtual clock moves only by `App::advance`, each `advance` runs
//! one frame.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use super::SMOOTH_SCROLL_DURATION;
use crate::TuiDom;
use crate::accessors::{
    ScrollBehaviorOption, ScrollIntoViewOptions, ScrollToOptions, TuiAccessors, TuiAccessorsMut,
};
use crate::layout::{Display, Overflow, ScrollBehavior, Size};
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const DURATION_MS: u64 = SMOOTH_SCROLL_DURATION.as_millis() as u64;

/// A 5-row scroll pane of 50 one-row lines (max `scrollTop` 45) with
/// the given `scroll-behavior`; returns the pane and its 41st line.
fn pane_app(behavior: ScrollBehavior) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = dom.create_element("div");
    dom.set_attribute(pane, "id", "pane").unwrap();
    dom.set_attribute(pane, "tabindex", "0").unwrap();
    dom.append_child(root, pane).unwrap();
    let mut line_40 = pane;
    for i in 0..50 {
        let line = dom.create_element("p");
        let t = dom.create_text_node(&format!("line {i}"));
        dom.append_child(line, t).unwrap();
        dom.append_child(pane, line).unwrap();
        if i == 40 {
            line_40 = line;
        }
    }
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "#pane",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .overflow_y(Overflow::Auto)
                .scroll_behavior(behavior),
        )
        .rule_unchecked("p", TuiStyle::new().display(Display::Block));
    let terminal = Terminal::new(TestBackend::new(30, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, pane, line_40)
}

fn top(app: &App<TestBackend>, pane: NodeId) -> i32 {
    app.dom().node(pane).scroll_top().unwrap()
}

/// Count the `scroll` events fired on `pane`.
fn count_scrolls(app: &mut App<TestBackend>, pane: NodeId) -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(0));
    let c = n.clone();
    app.dom_mut()
        .add_event_listener(pane, "scroll", ListenerOptions::default(), move |_| {
            c.set(c.get() + 1)
        })
        .unwrap();
    n
}

fn wheel_down(app: &mut App<TestBackend>) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 2,
        row: 2,
        modifiers: KeyModifiers::empty(),
    }));
}

#[test]
fn smooth_scroll_to_reaches_the_target_after_the_duration() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    let scrolls = count_scrolls(&mut app, pane);
    app.dom_mut().node_mut(pane).scroll_to(0, 40).unwrap();
    assert_eq!(top(&app, pane), 0, "a smooth scroll does not jump");
    app.advance(0).unwrap();
    assert_eq!(top(&app, pane), 0, "the animation starts at the next frame");

    let mut seen = Vec::new();
    for _ in 0..(DURATION_MS / 50) {
        app.advance(50).unwrap();
        seen.push(top(&app, pane));
    }
    assert_eq!(
        *seen.last().unwrap(),
        40,
        "at the target after the duration"
    );
    assert!(
        seen.iter().any(|&y| y > 0 && y < 40),
        "intermediate positions: {seen:?}"
    );
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "monotonic: {seen:?}");
    // Ease-out: the first step covers more ground than the last.
    assert!(seen[0] > 40 - seen[seen.len() - 2], "ease-out: {seen:?}");
    assert!(
        scrolls.get() >= 3,
        "`scroll` fires per step, got {}",
        scrolls.get()
    );
}

#[test]
fn auto_scroll_behavior_scrolls_instantly() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Auto);
    app.dom_mut().node_mut(pane).scroll_to(0, 40).unwrap();
    assert_eq!(top(&app, pane), 40);
    app.dom_mut().node_mut(pane).set_scroll_top(10).unwrap();
    assert_eq!(top(&app, pane), 10);
}

#[test]
fn set_scroll_top_and_scroll_by_honor_smooth() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut().node_mut(pane).set_scroll_top(20).unwrap();
    assert_eq!(top(&app, pane), 0);
    app.advance(0).unwrap();
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 20);
    app.dom_mut().node_mut(pane).scroll_by(0, 5).unwrap();
    assert_eq!(top(&app, pane), 20);
    app.advance(0).unwrap();
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 25);
}

#[test]
fn explicit_behavior_overrides_the_property() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut()
        .node_mut(pane)
        .scroll_with(ScrollToOptions {
            left: None,
            top: Some(30),
            behavior: ScrollBehaviorOption::Instant,
        })
        .unwrap();
    assert_eq!(top(&app, pane), 30, "`instant` jumps under `smooth`");

    let (mut app, pane, _) = pane_app(ScrollBehavior::Auto);
    app.dom_mut()
        .node_mut(pane)
        .scroll_by_with(ScrollToOptions {
            left: None,
            top: Some(30),
            behavior: ScrollBehaviorOption::Smooth,
        })
        .unwrap();
    assert_eq!(top(&app, pane), 0, "`smooth` animates under `auto`");
    app.advance(0).unwrap();
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 30);
}

#[test]
fn scroll_into_view_honors_smooth() {
    let (mut app, pane, line_40) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut().node_mut(line_40).scroll_into_view().unwrap();
    assert_eq!(top(&app, pane), 0);
    app.advance(0).unwrap();
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 40);

    let (mut app, pane, line_40) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut()
        .node_mut(line_40)
        .scroll_into_view_with(ScrollIntoViewOptions::new().behavior(ScrollBehaviorOption::Instant))
        .unwrap();
    assert_eq!(top(&app, pane), 40, "`instant` jumps");
}

/// `scrollIntoView` of a line already at the top of a scrolled pane
/// leaves the pane where it is: the line's position is measured from
/// the pane's content origin, not from its scrolled viewport.
#[test]
fn scroll_into_view_inside_a_scrolled_container_keeps_a_line_at_the_top() {
    let (mut app, pane, line_40) = pane_app(ScrollBehavior::Auto);
    app.dom_mut().node_mut(pane).scroll_to(0, 39).unwrap();
    // The wheel requests a frame, which lays the lines out at 40.
    wheel_down(&mut app);
    app.advance(0).unwrap();
    assert_eq!(top(&app, pane), 40);
    app.dom_mut().node_mut(line_40).scroll_into_view().unwrap();
    assert_eq!(top(&app, pane), 40);
}

#[test]
fn the_wheel_is_instant_even_under_smooth_and_aborts_a_smooth_scroll() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    wheel_down(&mut app);
    assert_eq!(top(&app, pane), 1, "the wheel scrolls instantly");

    app.dom_mut().node_mut(pane).scroll_to(0, 40).unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let mid = top(&app, pane);
    assert!(mid > 1 && mid < 40, "{mid}");
    wheel_down(&mut app);
    assert_eq!(top(&app, pane), mid + 1);
    app.advance(DURATION_MS).unwrap();
    assert_eq!(
        top(&app, pane),
        mid + 1,
        "a user scroll aborts the smooth scroll (CSSOM View: perform a scroll)"
    );
}

#[test]
fn a_new_request_retargets_from_the_current_position() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut().node_mut(pane).scroll_to(0, 40).unwrap();
    app.advance(0).unwrap();
    app.advance(DURATION_MS / 2).unwrap();
    let mid = top(&app, pane);
    assert!(mid > 10 && mid < 40, "{mid}");

    app.dom_mut().node_mut(pane).scroll_to(0, 10).unwrap();
    assert_eq!(top(&app, pane), mid, "retargeting does not jump");
    app.advance(0).unwrap();
    app.advance(DURATION_MS / 2).unwrap();
    let back = top(&app, pane);
    assert!(back < mid && back > 10, "heading back from {mid}: {back}");
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 10);
}

#[test]
fn a_settled_smooth_scroll_schedules_no_wakeups() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    assert_eq!(app.smooth_scroll_deadline(), None, "nothing in flight");
    app.dom_mut().node_mut(pane).scroll_to(0, 40).unwrap();
    app.advance(0).unwrap();
    assert!(app.smooth_scroll_deadline().is_some(), "in flight");
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 40);
    assert_eq!(app.smooth_scroll_deadline(), None, "settled");
    app.advance(DURATION_MS).unwrap();
    assert_eq!(app.smooth_scroll_deadline(), None);
}

#[test]
fn keyboard_scrolling_is_smooth_under_smooth() {
    let (mut app, pane, _) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut().set_focused(Some(pane));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::empty(),
    )));
    assert_eq!(top(&app, pane), 0);
    // A second PageDown while the first is in flight pages on from its
    // destination, as browsers' keyboard scrolling does.
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
    app.advance(DURATION_MS).unwrap();
    assert_eq!(top(&app, pane), 10);

    let (mut app, pane, _) = pane_app(ScrollBehavior::Auto);
    app.dom_mut().set_focused(Some(pane));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::empty(),
    )));
    assert_eq!(top(&app, pane), 5, "instant under `auto`");
}
