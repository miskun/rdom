//! C8G-RESNAP — re-snapping after layout (CSS Scroll Snap 1 §5.4) only
//! when the layout moved the snap position the container rests at; every
//! scroll that does not snap — a smooth scroll's steps, a thumb drag's
//! moves, autoscroll, a caret reveal — leaves the container unsnapped;
//! the re-snap visits only the containers that snapped and queues its
//! `scroll` event for after the frame, as HTML's "run the scroll steps"
//! does, instead of firing it between layout and paint.

use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{LayoutExt, Rect, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::CascadeExt;

const SHEET: &str = ".s { width: 10; height: 8; overflow-y: scroll; \
                     scroll-snap-type: y mandatory } \
                     .i { height: 3; scroll-snap-align: start }";

/// `.s` — a 10 × 8 mandatory snap container of ten 3-row `.i` items
/// (rows 0–30, positions 0, 3, …, 21 and the end 22): the dom, the
/// container and the items.
fn build() -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.append_child(root, s).unwrap();
    let items = (0..10)
        .map(|_| {
            let id = dom.create_element("div");
            dom.set_attribute(id, "class", "i").unwrap();
            dom.append_child(s, id).unwrap();
            id
        })
        .collect();
    (dom, s, items)
}

const AREA: Rect = Rect::new(0, 0, 12, 10);

/// [`build`], cascaded and laid out without an `App`.
fn laid_out() -> (TuiDom, NodeId, Vec<NodeId>) {
    let (mut dom, s, items) = build();
    dom.cascade(&rdom_css::from_css_strict(SHEET).unwrap());
    dom.layout_dom(AREA);
    (dom, s, items)
}

/// [`build`] styled `extra` on top of [`SHEET`], in an `App`, one frame
/// drawn.
fn app(extra: &str) -> (App<TestBackend>, NodeId, Vec<NodeId>) {
    let (dom, s, items) = build();
    let sheet = rdom_css::from_css_strict(&format!("{SHEET} {extra}")).unwrap();
    let terminal = Terminal::new(TestBackend::new(AREA.width, AREA.height)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, s, items)
}

fn top(app: &App<TestBackend>, s: NodeId) -> i32 {
    app.dom().node(s).scroll_top().unwrap()
}

/// Every `scroll_top` a `scroll` listener on `s` saw, in order.
fn record_scrolls(app: &mut App<TestBackend>, s: NodeId) -> Rc<RefCell<Vec<i32>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    app.dom_mut()
        .add_event_listener(s, "scroll", ListenerOptions::default(), move |ctx| {
            log.borrow_mut()
                .push(ctx.dom.node(s).scroll_top().unwrap_or(0));
        })
        .unwrap();
    seen
}

/// CSSOM View "perform a scroll" with `scroll-behavior: smooth` under a
/// mandatory snap: PageDown (8 rows from 0, directional) goes to 9 and
/// animates there — a frame mid-way shows an offset between, and the
/// `scroll` events never go back.
#[test]
fn a_smooth_page_down_animates_under_mandatory_snapping() {
    let (mut app, s, _) = app(".s { scroll-behavior: smooth }");
    let seen = record_scrolls(&mut app, s);
    app.dom_mut().set_focused(Some(s));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::empty(),
    )));
    app.advance(16).unwrap();
    app.advance(60).unwrap();
    let mid = top(&app, s);
    assert!(0 < mid && mid < 9, "mid-animation: {mid}");
    app.advance(300).unwrap();
    assert_eq!(top(&app, s), 9, "settled on the snap position");
    let seen = seen.borrow();
    assert!(
        seen.windows(2).all(|w| w[0] <= w[1]),
        "scroll events never went back: {seen:?}"
    );
    assert_eq!(seen.last(), Some(&9));
}

/// A thumb drag is not a snap: after `scrollTo` snapped the container to
/// 6, dragging the thumb moves it, and the frames during the drag keep the
/// dragged offset (the release snaps, `releasing_a_thumb_drag_snaps`).
#[test]
fn a_thumb_drag_after_a_snap_holds() {
    let (mut app, s, _) = app("");
    app.dom_mut().node_mut(s).scroll_to(0, 6).unwrap();
    app.advance(16).unwrap();
    assert_eq!(top(&app, s), 6);
    let mouse = |kind, row| {
        CtEvent::Mouse(MouseEvent {
            kind,
            column: 9,
            row,
            modifiers: KeyModifiers::empty(),
        })
    };
    // Track 8 rows for 30: the thumb is 2 rows at row 1 for offset 6.
    app.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), 1));
    app.handle_event(mouse(MouseEventKind::Drag(MouseButton::Left), 3));
    app.advance(16).unwrap();
    let dragged = top(&app, s);
    assert!(dragged > 6, "the drag moved it: {dragged}");
    app.advance(16).unwrap();
    assert_eq!(top(&app, s), dragged, "a later frame keeps the drag");
}

/// A scroll that is not a snap's leaves the container unsnapped: a
/// layout that then moves the box it was snapped to (an item inserted
/// before it, mid-drag) does not pull the dragged offset back to it.
#[test]
fn a_layout_change_mid_drag_keeps_the_drag() {
    let (mut app, s, items) = app("");
    app.dom_mut().node_mut(s).scroll_to(0, 6).unwrap();
    app.advance(16).unwrap();
    let mouse = |kind, row| {
        CtEvent::Mouse(MouseEvent {
            kind,
            column: 9,
            row,
            modifiers: KeyModifiers::empty(),
        })
    };
    app.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), 1));
    app.handle_event(mouse(MouseEventKind::Drag(MouseButton::Left), 3));
    app.advance(16).unwrap();
    let dragged = top(&app, s);
    let dom = app.dom_mut();
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    app.advance(16).unwrap();
    assert_eq!(top(&app, s), dragged);
}

/// §5.4 re-snaps only when the layout moved the snap position: a raw
/// offset write past the funnel (`TuiExt::scroll_y` is a public field)
/// keeps the snap record, and a layout that leaves the box where it was
/// still leaves the written offset alone.
#[test]
fn a_layout_that_does_not_move_the_target_leaves_the_offset() {
    let (mut dom, s, _) = laid_out();
    dom.node_mut(s).scroll_to(0, 6).unwrap();
    dom.layout_dom(AREA);
    dom.node_mut(s).ext_mut().unwrap().scroll_y = 8;
    dom.layout_dom(AREA);
    assert!(!super::resnap(&mut dom));
    assert_eq!(dom.node(s).scroll_top(), Some(8));
}

/// §5.4 still re-snaps when the layout moves the target: an item inserted
/// before the one at 6 moves it to 9 — and the `scroll` event that move
/// fires reaches its listener after the frame, not between layout and
/// paint: `resnap` itself dispatches nothing.
#[test]
fn a_resnap_queues_its_scroll_event() {
    let (mut dom, s, items) = laid_out();
    let fired = Rc::new(std::cell::Cell::new(0));
    let f = fired.clone();
    dom.add_event_listener(s, "scroll", ListenerOptions::default(), move |_| {
        f.set(f.get() + 1)
    })
    .unwrap();
    dom.node_mut(s).scroll_to(0, 6).unwrap();
    assert_eq!(fired.get(), 1, "scrollTo fires at once");
    dom.layout_dom(AREA);
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    dom.cascade(&rdom_css::from_css_strict(SHEET).unwrap());
    dom.layout_dom(AREA);
    assert!(super::resnap(&mut dom));
    assert_eq!(dom.node(s).scroll_top(), Some(9));
    assert_eq!(fired.get(), 1, "nothing fired mid-pipeline");
    assert_eq!(
        crate::runtime::scrollbar::take_queued_scroll_events(&mut dom),
        vec![s]
    );
}

/// The frame drains that queue after painting: the listener runs once.
#[test]
fn the_frame_fires_a_resnaps_scroll_event_after_painting() {
    let (mut app, s, items) = app("");
    app.dom_mut().node_mut(s).scroll_to(0, 6).unwrap();
    app.advance(16).unwrap();
    let seen = record_scrolls(&mut app, s);
    let dom = app.dom_mut();
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    app.advance(16).unwrap();
    assert_eq!(top(&app, s), 9);
    assert_eq!(*seen.borrow(), vec![9]);
}

/// Architect N4: a frame with no snapped container visits none — the
/// re-snap keeps the set of containers that snapped, not a tree walk.
#[test]
fn resnap_visits_only_snapped_containers() {
    let (mut dom, s, _) = laid_out();
    super::VISITS.with(|c| c.set(0));
    assert!(!super::resnap(&mut dom));
    assert_eq!(super::VISITS.with(|c| c.get()), 0, "nothing snapped yet");
    dom.node_mut(s).scroll_to(0, 6).unwrap();
    super::resnap(&mut dom);
    assert_eq!(super::VISITS.with(|c| c.get()), 1, "the one that snapped");
}
