//! Mouse-routing tests — exercise `Router::route` with synthetic
//! crossterm events against `Dom<TuiExt>` + test-only scaffolding.
//!
//! Covers down/up synthesis, click = common ancestor, and
//! mousemove auto-hover transitions.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};
use rdom_core::{AbortController, ListenerOptions, NodeId, Position, Selection};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::{Display, Overflow, Padding, Size, UserSelect};
use crate::render::{LayoutExt, Rect};
use crate::runtime::router::{RouteOutcome, Router};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Test helpers ────────────────────────────────────────────────────

fn mouse_at(kind: MouseEventKind, x: u16, y: u16) -> CtMouseEvent {
    CtMouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }
}

fn down_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::Down(MouseButton::Left), x, y)
}

fn up_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::Up(MouseButton::Left), x, y)
}

fn move_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::Moved, x, y)
}

fn prepare(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
}

/// A nestable "event recorder" — accumulates `(target, event_type)`
/// pairs so tests can assert dispatch order across nodes.
type Log = Rc<RefCell<Vec<(NodeId, String)>>>;

fn log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

fn record(dom: &mut TuiDom, node: NodeId, event_type: &str, log: &Log) {
    let log = log.clone();
    let ty = event_type.to_string();
    dom.add_event_listener(node, event_type, ListenerOptions::default(), move |ctx| {
        log.borrow_mut()
            .push((ctx.event.current_target.unwrap_or(node), ty.clone()));
    })
    .unwrap();
}

// ── scroll (M5 D5) ──────────────────────────────────────────────────

#[test]
fn wheel_that_scrolls_dispatches_scroll_event() {
    // M5 D5: a wheel event that advances scroll_y dispatches a
    // `scroll` event on the scrolled element.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("scroller");
    dom.append_child(root, scroller).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "scroller",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5))
            .overflow(Overflow::Auto),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 30, 10));
    // Inflate the content so the scroller is actually scrollable.
    if let Some(ext) = dom.node_mut(scroller).ext_mut() {
        ext.scroll_content_height = 50;
    }

    let log = log();
    record(&mut dom, scroller, "scroll", &log);

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::ScrollDown, 5, 2)),
    );

    assert_eq!(
        log.borrow().len(),
        1,
        "scroll fires once per wheel tick that changed offset"
    );
    assert_eq!(log.borrow()[0].1, "scroll");
}

#[test]
fn wheel_with_no_scrollable_offset_change_does_not_fire_scroll() {
    // At scroll_top = max_y already, an additional wheel-down tick
    // is a no-op (saturating clamp). No scroll event fires.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("scroller");
    dom.append_child(root, scroller).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "scroller",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5))
            .overflow(Overflow::Auto),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 30, 10));
    // No content overflow → max_y = 0, wheel cannot scroll.
    if let Some(ext) = dom.node_mut(scroller).ext_mut() {
        ext.scroll_content_height = 3; // less than viewport
    }

    let log = log();
    record(&mut dom, scroller, "scroll", &log);

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::ScrollDown, 5, 2)),
    );

    assert!(
        log.borrow().is_empty(),
        "no offset change = no scroll event"
    );
}

// ── contextmenu (M5 D2) ─────────────────────────────────────────────

#[test]
fn right_mousedown_dispatches_contextmenu_on_hit_target() {
    // Right-button down fires `contextmenu` at the hit target.
    // Cancelable; bubbles up the ancestor chain.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "contextmenu", &log);

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Down(MouseButton::Right), 2, 1)),
    );

    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].1, "contextmenu");
}

#[test]
fn left_mousedown_does_not_fire_contextmenu() {
    // Regression guard: left-button keeps existing behavior; the
    // new contextmenu code path must not fire on left clicks.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "contextmenu", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 1)));

    assert!(
        log.borrow().is_empty(),
        "left click must not fire contextmenu"
    );
}

#[test]
fn contextmenu_off_screen_does_not_fire() {
    // No hit target → no contextmenu dispatch (matches the
    // existing mousedown behavior on miss).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "contextmenu", &log);

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Down(MouseButton::Right), 50, 50)),
    );

    assert!(log.borrow().is_empty());
}

// ── dblclick (M5 D3) ────────────────────────────────────────────────

#[test]
fn second_click_dispatches_dblclick() {
    // M5 D3: two clicks at the same position within the multi-click
    // window dispatch `dblclick` on the second click, in addition
    // to the second `click`. Matches HTML.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "click", &log);
    record(&mut dom, el, "dblclick", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));

    let events: Vec<String> = log.borrow().iter().map(|(_, t)| t.clone()).collect();
    // Expected: click, click, dblclick (dblclick fires AFTER the
    // second click, matching HTML).
    assert_eq!(
        events.iter().filter(|t| t.as_str() == "click").count(),
        2,
        "two clicks fired"
    );
    assert_eq!(
        events.iter().filter(|t| t.as_str() == "dblclick").count(),
        1,
        "exactly one dblclick fired"
    );
    let click_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (t.as_str() == "click").then_some(i))
        .collect();
    let dblclick_position = events
        .iter()
        .position(|t| t.as_str() == "dblclick")
        .unwrap();
    assert!(
        dblclick_position > click_positions[1],
        "dblclick fires AFTER the second click (got order: {events:?})"
    );
}

#[test]
fn single_click_does_not_fire_dblclick() {
    // Sanity: a lone click never produces a dblclick.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "dblclick", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));

    assert!(log.borrow().is_empty());
}

#[test]
fn triple_click_fires_dblclick_only_once() {
    // M5 D3: dblclick fires on the SECOND click of a sequence.
    // The third click does not produce another dblclick (the next
    // dblclick would require a fresh pair starting from a 4th
    // click, since register_click resets after 3).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    let log = log();
    record(&mut dom, el, "dblclick", &log);

    let mut router = Router::new();
    for _ in 0..3 {
        router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
        router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));
    }

    assert_eq!(
        log.borrow().len(),
        1,
        "dblclick fires once across a triple-click"
    );
}

// ── mousedown ───────────────────────────────────────────────────────

#[test]
fn mousedown_on_hit_dispatches_mousedown_event() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "mousedown", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));

    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].0, div);
    assert_eq!(log.borrow()[0].1, "mousedown");
    assert_eq!(router.down_target(), Some(div));
}

#[test]
fn mousedown_miss_records_no_target() {
    let mut dom: TuiDom = TuiDom::new();
    prepare(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(5, 5)));
    assert_eq!(router.down_target(), None);
}

// ── mouseup + click synthesis ───────────────────────────────────────

#[test]
fn mouseup_same_target_as_down_synthesizes_click() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("btn");
    dom.append_child(root, btn).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "btn",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, btn, "mousedown", &log);
    record(&mut dom, btn, "mouseup", &log);
    record(&mut dom, btn, "click", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));

    let events: Vec<_> = log.borrow().iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(events, vec!["mousedown", "mouseup", "click"]);
    assert_eq!(router.down_target(), None, "down_target cleared on mouseup");
}

#[test]
fn mouseup_different_child_of_same_parent_click_fires_on_parent() {
    // Target A = child a; target B = child b. common ancestor =
    // parent. Click dispatches on parent.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "parent",
            TuiStyle::new()
                .direction(crate::layout::Direction::Row)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, a, "click", &log);
    record(&mut dom, b, "click", &log);
    record(&mut dom, parent, "click", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 1))); // in a
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(7, 1))); // in b

    // Click targeting parent — bubbles up to root listener too. a and
    // b don't get direct clicks (click was synthesized on parent), but
    // bubbling means their own listeners wouldn't fire either (target
    // is parent, dispatch bubbles parent → root).
    let events: Vec<_> = log.borrow().iter().map(|(n, e)| (*n, e.clone())).collect();
    assert_eq!(events, vec![(parent, "click".to_string())]);
}

#[test]
fn mouseup_without_prior_mousedown_does_not_synthesize_click() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "mouseup", &log);
    record(&mut dom, div, "click", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));

    let events: Vec<_> = log.borrow().iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(events, vec!["mouseup"]); // no click
}

#[test]
fn mouseup_after_drag_off_viewport_no_click() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "click", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 1)));
    // Release far off div.
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(18, 8)));

    assert!(log.borrow().is_empty());
    assert_eq!(router.down_target(), None);
}

#[test]
fn click_event_is_marked_synthetic() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let captured = Rc::new(RefCell::new(false));
    let c2 = captured.clone();
    dom.add_event_listener(div, "click", ListenerOptions::default(), move |ctx| {
        *c2.borrow_mut() = ctx.event.is_synthetic();
    })
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 1)));

    assert!(
        *captured.borrow(),
        "synthesized click carries is_synthetic=true"
    );
}

// ── mousemove + hover transitions ───────────────────────────────────

#[test]
fn mousemove_onto_element_fires_mouseover_and_sets_hovered() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "mouseover", &log);

    let mut router = Router::new();
    let out = router.route(&mut dom, crossterm::event::Event::Mouse(move_at(3, 1)));

    let events: Vec<_> = log.borrow().iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(events, vec!["mouseover"]);
    assert_eq!(router.hover_target(), Some(div));
    assert_eq!(dom.hovered(), Some(div));
    assert!(out.redraw_requested, "hover transition requests redraw");
}

#[test]
fn mousemove_between_siblings_fires_mouseout_then_mouseover() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "parent",
            TuiStyle::new()
                .direction(crate::layout::Direction::Row)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, a, "mouseover", &log);
    record(&mut dom, a, "mouseout", &log);
    record(&mut dom, b, "mouseover", &log);
    record(&mut dom, b, "mouseout", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(2, 1))); // onto a
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(7, 1))); // onto b

    let events: Vec<_> = log.borrow().iter().map(|(n, e)| (*n, e.clone())).collect();
    assert_eq!(
        events,
        vec![
            (a, "mouseover".to_string()),
            (a, "mouseout".to_string()),
            (b, "mouseover".to_string()),
        ]
    );
    assert_eq!(router.hover_target(), Some(b));
    assert_eq!(dom.hovered(), Some(b));
}

#[test]
fn mousemove_same_target_does_not_refire_hover_events() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "mouseover", &log);
    record(&mut dom, div, "mouseout", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(2, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(5, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(7, 1)));

    // One mouseover on initial entry; no refires during movement
    // within the same element.
    let events: Vec<_> = log.borrow().iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(events, vec!["mouseover"]);
}

#[test]
fn mousemove_off_all_elements_fires_mouseout() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, div, "mouseout", &log);

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(2, 1))); // onto
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(18, 8))); // off

    let events: Vec<_> = log.borrow().iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(events, vec!["mouseout"]);
    assert_eq!(router.hover_target(), None);
    assert_eq!(dom.hovered(), None);
}

#[test]
fn hover_transition_events_are_marked_synthetic() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let over_syn = Rc::new(RefCell::new(false));
    let os = over_syn.clone();
    dom.add_event_listener(div, "mouseover", ListenerOptions::default(), move |ctx| {
        *os.borrow_mut() = ctx.event.is_synthetic();
    })
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(3, 1)));
    assert!(*over_syn.borrow());
}

// ── AbortSignal integration ─────────────────────────────────────────

#[test]
fn abort_signal_removes_mouse_listener_before_subsequent_event() {
    // End-to-end check: adding a listener governed by an
    // AbortSignal, aborting, and verifying it doesn't fire on the
    // next router dispatch.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let fired = Rc::new(RefCell::new(0usize));
    let f = fired.clone();
    let ctrl = AbortController::new();
    dom.add_event_listener(
        div,
        "mousedown",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| {
            *f.borrow_mut() += 1;
        },
    )
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    assert_eq!(*fired.borrow(), 1);

    ctrl.abort();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    assert_eq!(*fired.borrow(), 1); // not refired
}

// ── Overflow + hit-test interaction ─────────────────────────────────

#[test]
fn mousedown_on_overflow_hidden_padding_hits_container() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .padding(Padding::all(1))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let log = log();
    record(&mut dom, container, "mousedown", &log);
    record(&mut dom, child, "mousedown", &log);

    let mut router = Router::new();
    // (0, 0) is in container's padding — child is not hit.
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(0, 0)));

    let events: Vec<_> = log.borrow().iter().map(|(n, e)| (*n, e.clone())).collect();
    assert_eq!(events, vec![(container, "mousedown".to_string())]);
}

// ── Router state discipline ─────────────────────────────────────────

#[test]
fn reset_clears_all_state() {
    let mut router = Router::new();
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(3, 1)));
    assert!(router.down_target().is_some());
    assert!(router.hover_target().is_some());

    router.reset();
    assert!(router.down_target().is_none());
    assert!(router.hover_target().is_none());
}

#[test]
fn non_mouse_events_return_empty_outcome() {
    // Key events etc. are handled by `App`, not the router.
    let mut dom: TuiDom = TuiDom::new();
    let mut router = Router::new();
    let out = router.route(&mut dom, crossterm::event::Event::Resize(40, 20));
    assert_eq!(out, RouteOutcome::default());
}

// ── Wheel scrolling ─────────────────────────────────────────────────

fn scroll_up_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::ScrollUp, x, y)
}

fn scroll_down_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::ScrollDown, x, y)
}

fn scroll_left_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::ScrollLeft, x, y)
}

fn scroll_right_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::ScrollRight, x, y)
}

#[test]
fn wheel_on_scrollable_ancestor_increments_scroll_y() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();

    // Child taller than the 5-row container so scrolling is
    // actually possible. Without overflow, the max-clamp keeps
    // scroll_y at 0 — DOM-correct, since there's nowhere to
    // scroll to.
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(20))
                .flex_shrink(0),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));
    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 0);

    let mut router = Router::new();
    let out = router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(3, 1)),
    );

    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 1);
    assert!(out.redraw_requested);
}

#[test]
fn wheel_scroll_up_decrements_scroll_y_but_clamps_at_zero() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(20))
                .flex_shrink(0),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Pre-seed scroll_y so we can observe both decrement and clamp.
    dom.node_mut(container).ext_mut().unwrap().scroll_y = 2;

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(scroll_up_at(3, 1)));
    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 1);

    // Wheel up twice more → should clamp at 0, not underflow.
    router.route(&mut dom, crossterm::event::Event::Mouse(scroll_up_at(3, 1)));
    router.route(&mut dom, crossterm::event::Event::Mouse(scroll_up_at(3, 1)));
    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 0);
}

/// Wheel-down clamps at `scroll_content_height - viewport_height` —
/// can't scroll past the end of the content. Regression guard: an
/// unclamped offset would let the thumb glue to the bottom of the
/// track without moving any content.
#[test]
fn wheel_scroll_down_clamps_at_content_end() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    // Container is 10 wide × 5 tall. Child is 5 wide × 8 tall →
    // 3 cells of vertical overflow. Wheel-down 10 times should
    // clamp at scroll_y = 3 (no further movement).
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow_y(Overflow::Scroll),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(8))
                .flex_shrink(0),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // scroll_content_height = 8, viewport content_layout.height =
    // 5 (no horizontal gutter reserved for overflow_y-only) →
    // max_y = 3.
    let mut router = Router::new();
    for _ in 0..10 {
        router.route(
            &mut dom,
            crossterm::event::Event::Mouse(scroll_down_at(3, 1)),
        );
    }
    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 3);
}

/// Wheel-down on a container whose content fits the viewport is
/// a no-op — max_y = 0, scroll_y stays at 0. Without the upper
/// clamp this would erroneously increment, painting the scrollbar
/// thumb in a position that doesn't correspond to any content.
#[test]
fn wheel_scroll_down_is_noop_when_content_fits_viewport() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(10))
                .overflow_y(Overflow::Scroll),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 12));

    let mut router = Router::new();
    for _ in 0..5 {
        router.route(
            &mut dom,
            crossterm::event::Event::Mouse(scroll_down_at(3, 1)),
        );
    }
    assert_eq!(dom.node(container).ext().unwrap().scroll_y, 0);
}

#[test]
fn wheel_horizontal_scroll_adjusts_scroll_x() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    // Child wider than the 10-wide container so horizontal
    // scroll is actually possible.
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .width(Size::Fixed(30))
                .height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_right_at(3, 1)),
    );
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_right_at(3, 1)),
    );
    assert_eq!(dom.node(container).ext().unwrap().scroll_x, 2);

    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_left_at(3, 1)),
    );
    assert_eq!(dom.node(container).ext().unwrap().scroll_x, 1);
}

#[test]
fn wheel_walks_up_to_find_scrollable_ancestor() {
    // A nested child without its own overflow should scroll the
    // containing scrollable parent.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("scroller");
    let inner = dom.create_element("inner");
    let leaf = dom.create_element("leaf");
    dom.append_child(inner, leaf).unwrap();
    dom.append_child(scroller, inner).unwrap();
    dom.append_child(root, scroller).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scroller",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "inner",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .height(Size::Fixed(20))
                .flex_shrink(0),
        )
        .rule_unchecked(
            "leaf",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(2, 1)),
    );

    assert_eq!(dom.node(scroller).ext().unwrap().scroll_y, 1);
    // Neither descendant scrolled — only the overflow container.
    assert_eq!(dom.node(inner).ext().unwrap().scroll_y, 0);
    assert_eq!(dom.node(leaf).ext().unwrap().scroll_y, 0);
}

#[test]
fn wheel_without_any_scrollable_ancestor_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    let out = router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(3, 1)),
    );
    assert_eq!(dom.node(div).ext().unwrap().scroll_y, 0);
    assert!(!out.redraw_requested);
}

#[test]
fn wheel_prevent_default_skips_auto_scroll() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(container, child).unwrap();
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Handler on container cancels default scrolling.
    let got = Rc::new(Cell::new(false));
    let g = got.clone();
    dom.add_event_listener(
        container,
        "wheel",
        rdom_core::ListenerOptions::default(),
        move |ctx| {
            ctx.event.prevent_default();
            g.set(true);
        },
    )
    .unwrap();

    let mut router = Router::new();
    let out = router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(3, 1)),
    );

    assert!(got.get(), "handler ran");
    assert_eq!(
        dom.node(container).ext().unwrap().scroll_y,
        0,
        "default action skipped"
    );
    assert!(!out.redraw_requested);
}

#[test]
fn wheel_bubbles_up_to_ancestor_handlers() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "inner",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(20))
                .flex_shrink(0),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let outer_fired = Rc::new(Cell::new(false));
    let o = outer_fired.clone();
    dom.add_event_listener(
        outer,
        "wheel",
        rdom_core::ListenerOptions::default(),
        move |_| {
            o.set(true);
        },
    )
    .unwrap();

    let mut router = Router::new();
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(2, 1)),
    );
    // Event dispatched on inner bubbles up to outer.
    assert!(outer_fired.get());
    // Default action still ran — scroll incremented on outer.
    assert_eq!(dom.node(outer).ext().unwrap().scroll_y, 1);
}

#[test]
fn wheel_outside_viewport_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    prepare(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 10));
    let mut router = Router::new();
    let out = router.route(
        &mut dom,
        crossterm::event::Event::Mouse(scroll_down_at(99, 99)),
    );
    assert_eq!(out, RouteOutcome::default());
}

// ── Pointer capture ─────────────────────────────────────────────────

fn drag_at(x: u16, y: u16) -> CtMouseEvent {
    mouse_at(MouseEventKind::Drag(MouseButton::Left), x, y)
}

/// Build a two-element row layout where `handle` occupies
/// columns 0..5 and `rest` occupies columns 5..20, both on row
/// 0..3. A drag from inside `handle` past x=5 ends up over `rest`
/// (or beyond) — exactly the "drag-select / resize-handle"
/// scenario pointer capture is meant to handle.
fn drag_handle_fixture() -> (TuiDom, NodeId, NodeId) {
    use crate::layout::Direction;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("container");
    let handle = dom.create_element("handle");
    let rest = dom.create_element("rest");
    dom.append_child(container, handle).unwrap();
    dom.append_child(container, rest).unwrap();
    dom.append_child(root, container).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "container",
            TuiStyle::new()
                .direction(Direction::Row)
                .width(Size::Fixed(20))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "handle",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "rest",
            TuiStyle::new()
                .width(Size::Fixed(15))
                .height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));
    (dom, handle, rest)
}

#[test]
fn pointer_capture_routes_mousemove_to_captured_regardless_of_hit() {
    let (mut dom, handle, _rest) = drag_handle_fixture();

    let moves_on_handle = Rc::new(Cell::new(0));
    let m = moves_on_handle.clone();
    dom.add_event_listener(handle, "mousemove", ListenerOptions::default(), move |_| {
        m.set(m.get() + 1);
    })
    .unwrap();

    dom.set_pointer_capture(handle).unwrap();

    let mut router = Router::new();
    // Move well beyond the handle's rect; without capture this would
    // hit `rest` and dispatch mousemove there.
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(12, 1)));

    assert_eq!(moves_on_handle.get(), 1);
}

#[test]
fn pointer_capture_suppresses_hover_transitions() {
    let (mut dom, handle, rest) = drag_handle_fixture();

    let over_handle = Rc::new(Cell::new(0));
    let o = over_handle.clone();
    dom.add_event_listener(handle, "mouseover", ListenerOptions::default(), move |_| {
        o.set(o.get() + 1);
    })
    .unwrap();
    let over_rest = Rc::new(Cell::new(0));
    let r = over_rest.clone();
    dom.add_event_listener(rest, "mouseover", ListenerOptions::default(), move |_| {
        r.set(r.get() + 1);
    })
    .unwrap();

    dom.set_pointer_capture(handle).unwrap();

    let mut router = Router::new();
    // Without capture: would fire mouseover on rest.
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(12, 1)));

    assert_eq!(over_handle.get(), 0, "no mouseover during capture");
    assert_eq!(over_rest.get(), 0, "no mouseover during capture");
    assert_eq!(router.hover_target(), None, "hover_target untouched");
    assert_eq!(dom.hovered(), None, "dom.hovered() untouched");
}

#[test]
fn drag_event_kind_also_routes_to_captured() {
    let (mut dom, handle, _rest) = drag_handle_fixture();

    let moves = Rc::new(Cell::new(0));
    let m = moves.clone();
    dom.add_event_listener(handle, "mousemove", ListenerOptions::default(), move |_| {
        m.set(m.get() + 1);
    })
    .unwrap();

    dom.set_pointer_capture(handle).unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(drag_at(18, 2)));

    assert_eq!(moves.get(), 1);
}

#[test]
fn pointer_capture_routes_mouseup_to_captured() {
    let (mut dom, handle, _rest) = drag_handle_fixture();

    let ups_on_handle = Rc::new(Cell::new(0));
    let u = ups_on_handle.clone();
    dom.add_event_listener(handle, "mouseup", ListenerOptions::default(), move |_| {
        u.set(u.get() + 1);
    })
    .unwrap();

    dom.set_pointer_capture(handle).unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(12, 1)));

    assert_eq!(ups_on_handle.get(), 1);
}

#[test]
fn pointer_capture_auto_releases_on_mouseup() {
    let (mut dom, handle, _rest) = drag_handle_fixture();
    dom.set_pointer_capture(handle).unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(12, 1)));

    assert_eq!(dom.pointer_capture(), None);
}

#[test]
fn click_while_captured_targets_captured_not_common_ancestor() {
    let (mut dom, handle, rest) = drag_handle_fixture();

    let clicks_on_handle = Rc::new(Cell::new(0));
    let h = clicks_on_handle.clone();
    dom.add_event_listener(handle, "click", ListenerOptions::default(), move |_| {
        h.set(h.get() + 1);
    })
    .unwrap();
    let clicks_on_rest = Rc::new(Cell::new(0));
    let r = clicks_on_rest.clone();
    dom.add_event_listener(rest, "click", ListenerOptions::default(), move |_| {
        r.set(r.get() + 1);
    })
    .unwrap();

    // Simulate a real drag: mousedown on handle, capture, drag to
    // rest, mouseup over rest. Without capture, click would target
    // the parent (common ancestor). With capture, click = handle.
    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 1))); // on handle
    dom.set_pointer_capture(handle).unwrap();
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(12, 1))); // over rest

    assert_eq!(clicks_on_handle.get(), 1);
    assert_eq!(clicks_on_rest.get(), 0);
}

#[test]
fn set_pointer_capture_in_mousedown_handler_persists_through_drag() {
    // The real-world usage pattern: a mousedown listener claims
    // the pointer, then drag / mouseup all route to the listener's
    // element.
    let (mut dom, handle, _rest) = drag_handle_fixture();

    let handle_id_for_listener = handle;
    dom.add_event_listener(
        handle,
        "mousedown",
        ListenerOptions::default(),
        move |ctx| {
            // Use ctx.dom to claim the pointer for this element.
            ctx.dom.set_pointer_capture(handle_id_for_listener).unwrap();
        },
    )
    .unwrap();

    let received = Rc::new(Cell::new(0));
    let rc = received.clone();
    dom.add_event_listener(handle, "mousemove", ListenerOptions::default(), move |_| {
        rc.set(rc.get() + 1);
    })
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 1)));
    assert_eq!(dom.pointer_capture(), Some(handle));

    // Drag off the handle — mousemove should still route to handle.
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(15, 2)));
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(18, 2)));
    assert_eq!(received.get(), 2);

    // Mouseup clears capture.
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(18, 2)));
    assert_eq!(dom.pointer_capture(), None);
}

#[test]
fn explicit_release_pointer_capture_restores_normal_routing() {
    let (mut dom, handle, rest) = drag_handle_fixture();
    dom.set_pointer_capture(handle).unwrap();

    let hover_on_rest = Rc::new(Cell::new(false));
    let r = hover_on_rest.clone();
    dom.add_event_listener(rest, "mouseover", ListenerOptions::default(), move |_| {
        r.set(true);
    })
    .unwrap();

    // While captured, hover over rest is suppressed.
    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(12, 1)));
    assert!(!hover_on_rest.get());

    // Release and move again — hover should now update normally.
    dom.release_pointer_capture();
    router.route(&mut dom, crossterm::event::Event::Mouse(move_at(12, 1)));
    assert!(hover_on_rest.get());
}

// ── drag-select ─────────────────────────────────────────────────────

/// A paragraph with plain text "hello" at cells 0..5 on row 0,
/// inside a 20×10 viewport. An empty `<span>` sibling is appended
/// so `is_ifc_block` recognizes the `<p>` as an IFC (policy:
/// requires ≥1 inline element child, per `layout_pass::ifc`).
/// Returns (dom, p, text_node).
fn drag_text_fixture() -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));
    (dom, p, t)
}

#[test]
fn mousedown_on_text_starts_drag_and_sets_caret_selection() {
    let (mut dom, p, t) = drag_text_fixture();
    let mut router = Router::new();

    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));

    // Caret selection at byte 2 in the text node.
    let sel = dom.selection().expect("selection set on mousedown");
    assert!(sel.is_collapsed());
    assert_eq!(sel.anchor, Position::new(t, 2));

    // Pointer capture held on the IFC block (the <p>).
    assert_eq!(dom.pointer_capture(), Some(p));

    // Router knows a drag is in progress.
    assert_eq!(router.selection_drag, Some(p));
}

#[test]
fn mousedown_on_non_text_does_not_start_drag() {
    // A bare <div> with no text children — no IFC, so position_at
    // returns None and no drag begins.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 1)));

    assert!(dom.selection().is_none());
    assert_eq!(dom.pointer_capture(), None);
    assert_eq!(router.selection_drag, None);
}

#[test]
fn drag_extends_selection_focus_forward() {
    let (mut dom, _p, t) = drag_text_fixture();
    let mut router = Router::new();

    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(1, 0)));
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 4, 0)),
    );

    let sel = dom.selection().expect("selection still present");
    assert_eq!(sel.anchor, Position::new(t, 1));
    assert_eq!(sel.focus, Position::new(t, 4));
    assert!(!sel.is_collapsed());
}

#[test]
fn drag_extend_out_of_bounds_clamps_to_anchor_ifc_end() {
    // Browser behavior: dragging the mouse OUTSIDE any element
    // doesn't freeze the selection — it clamps to the nearest valid
    // position inside the drag's anchor inline-flow container. So
    // dragging past the bottom of a paragraph extends selection to
    // the paragraph's last position. Previously rdom froze focus at
    // the anchor, which silently missed text the user was trying to
    // select.
    let (mut dom, _p, t) = drag_text_fixture();
    let mut router = Router::new();

    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(1, 0)));
    // Drag into row 5 — empty terminal below the paragraph. No IFC
    // there, so position_at returns None; clamp_to_anchor_ifc kicks
    // in and clamps focus to the end of the anchor's last line.
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 1, 5)),
    );

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 1));
    // Text node `t` is "hello" (5 chars). Drag past bottom → focus
    // clamps to end of the last (only) fragment = offset 5.
    assert_eq!(sel.focus, Position::new(t, 5));
}

#[test]
fn backward_drag_preserves_anchor_before_focus() {
    let (mut dom, _p, t) = drag_text_fixture();
    let mut router = Router::new();

    // Start at cell 4, drag left to cell 1. Anchor stays at 4,
    // focus moves to 1 — the Selection preserves direction.
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(4, 0)));
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 1, 0)),
    );

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 4));
    assert_eq!(sel.focus, Position::new(t, 1));
}

#[test]
fn mouseup_ends_drag_and_releases_capture() {
    let (mut dom, _p, _t) = drag_text_fixture();
    let mut router = Router::new();

    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(1, 0)));
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 3, 0)),
    );
    router.route(&mut dom, crossterm::event::Event::Mouse(up_at(3, 0)));

    // Selection survives the mouseup (user expects their highlight
    // to stay after releasing the button).
    assert!(dom.selection().is_some());
    // But capture + drag state are cleared.
    assert_eq!(dom.pointer_capture(), None);
    assert_eq!(router.selection_drag, None);
}

#[test]
fn mousedown_on_user_select_none_does_not_start_drag() {
    // Chrome-like element: button text with user-select:none. A
    // click inside should focus/click-as-usual but NOT begin a
    // text-selection drag.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .user_select(UserSelect::None),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));

    assert!(dom.selection().is_none());
    assert_eq!(dom.pointer_capture(), None);
    assert_eq!(router.selection_drag, None);
}

#[test]
fn prevent_default_on_mousedown_suppresses_drag_begin() {
    let (mut dom, p, _t) = drag_text_fixture();

    // Handler cancels every default action (focus + drag-select).
    dom.add_event_listener(p, "mousedown", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));

    assert!(dom.selection().is_none());
    assert_eq!(dom.pointer_capture(), None);
    assert_eq!(router.selection_drag, None);
}

#[test]
fn drag_between_two_paragraphs_follows_innermost_ifc() {
    // Two paragraphs stacked — drag starts in p1. Continuing the
    // drag onto p2's row returns a position in p2; selection's
    // anchor stays in p1, focus moves into p2.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p1 = dom.create_element("p");
    let t1 = dom.create_text_node("abcde");
    dom.append_child(p1, t1).unwrap();
    let s1 = dom.create_element("span");
    dom.append_child(p1, s1).unwrap();
    let p2 = dom.create_element("p");
    let t2 = dom.create_text_node("fghij");
    dom.append_child(p2, t2).unwrap();
    let s2 = dom.create_element("span");
    dom.append_child(p2, s2).unwrap();
    dom.append_child(root, p1).unwrap();
    dom.append_child(root, p2).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(1, 0)));
    assert_eq!(router.selection_drag, Some(p1));

    // Drag into row 1 — p2's row. Capture is still on p1, but
    // position_at on row 1 finds p2 and maps into t2.
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 2, 1)),
    );

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 1));
    assert_eq!(sel.focus, Position::new(t2, 2));
}

#[test]
fn reset_clears_selection_drag_state() {
    let (mut dom, p, _t) = drag_text_fixture();
    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));
    assert_eq!(router.selection_drag, Some(p));

    router.reset();
    assert_eq!(router.selection_drag, None);
}

#[test]
fn selection_setter_preserved_when_handler_re_sets_on_mousedown() {
    // Regression guard: if a handler writes to dom.selection inside
    // its mousedown listener, the default drag-begin action runs
    // after and overwrites with a fresh caret. This mirrors the
    // browser ordering — default action is last.
    let (mut dom, _p, t) = drag_text_fixture();

    dom.add_event_listener(t, "mousedown", ListenerOptions::default(), {
        move |ctx| {
            // Set a nonsense selection from a handler — drag-begin
            // should replace it.
            ctx.dom
                .set_selection(Some(Selection::caret(Position::new(t, 99))));
        }
    })
    .unwrap();

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 2));
}

// ── user-select: all / contain (B1) ──────────────────────────────

#[test]
fn mousedown_inside_user_select_all_selects_whole_element() {
    // Browser behavior: a single click anywhere inside a
    // `user-select: all` element selects its entire text content as
    // one unit — for one-click-copy of tokens, URLs, code snippets.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("token-abc");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "p",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(15))
            .user_select(UserSelect::All),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    // Click at cell 3 (would be a caret at offset 3 under user-
    // select: text). With user-select: all, selection covers the
    // whole text node.
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(3, 0)));

    let sel = dom.selection().expect("selection set on click");
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, "token-abc".len()));
    assert!(!sel.is_collapsed());
}

#[test]
fn drag_inside_user_select_all_keeps_whole_element_selected() {
    // Once user-select: all has expanded the selection, dragging
    // shouldn't shrink it back to a caret — the all-host stays
    // fully selected for the duration of the drag.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("token-abc");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "p",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(15))
            .user_select(UserSelect::All),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 5, 0)),
    );

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, "token-abc".len()));
}

#[test]
fn drag_extend_clamps_to_user_select_contain_boundary() {
    // Browser behavior: `user-select: contain` traps a selection
    // inside the element where the drag started. Dragging out
    // doesn't let the focus escape — it clamps to the contain
    // host's nearest valid position.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p1 = dom.create_element("p");
    let t1 = dom.create_text_node("abcde");
    dom.append_child(p1, t1).unwrap();
    let p2 = dom.create_element("p");
    let t2 = dom.create_text_node("fghij");
    dom.append_child(p2, t2).unwrap();
    dom.append_child(root, p1).unwrap();
    dom.append_child(root, p2).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "p",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(10))
            .user_select(UserSelect::Contain),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let mut router = Router::new();
    // Click in p1 at offset 2, then drag down into p2 — focus
    // should clamp to the end of p1 (length 5), not move into t2.
    router.route(&mut dom, crossterm::event::Event::Mouse(down_at(2, 0)));
    router.route(
        &mut dom,
        crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), 2, 1)),
    );

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 2));
    assert_eq!(
        sel.focus,
        Position::new(t1, "abcde".len()),
        "user-select: contain must clamp focus to its host's end; got {:?}",
        sel.focus
    );
    // t2 must NOT be involved.
    assert_ne!(sel.focus.node, t2);
}
