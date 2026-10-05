//! C8-SNAP — scroll snapping (CSS Scroll Snap 1) after the wheel, the
//! keyboard, a scrollbar drag, `scrollTo` / `scrollBy` /
//! `scrollIntoView` (a smooth one lands snapped), and after a layout
//! change (§5.4).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{LayoutExt, Rect};
use crate::runtime::router::Router;
use crate::runtime::scrollbar::handle_scroll_key;
use crate::style::CascadeExt;

const AREA: Rect = Rect::new(0, 0, 12, 8);

/// `.s` — a 10 × 4 scroller (`overflow-y: scroll`, styled `s`) of five
/// `.i` items 3 rows tall each (styled `i`): content rows 0–15, items at
/// 0, 3, 6, 9, 12; `scrollTop` 0 ..= 11.
fn scroller(s: &str, i: &str) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let sc = dom.create_element("div");
    dom.set_attribute(sc, "class", "s").unwrap();
    dom.append_child(root, sc).unwrap();
    let items = (0..5)
        .map(|_| {
            let id = dom.create_element("div");
            dom.set_attribute(id, "class", "i").unwrap();
            dom.append_child(sc, id).unwrap();
            id
        })
        .collect();
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ width: 10; height: 4; overflow-y: scroll; {s} }} .i {{ height: 3; {i} }}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    (dom, sc, items)
}

const MANDATORY: &str = "scroll-snap-type: y mandatory";
const START: &str = "scroll-snap-align: start";

fn top(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).scroll_top().unwrap()
}

fn wheel(dom: &mut TuiDom, kind: MouseEventKind) {
    let mut router = Router::new();
    router.route(
        dom,
        Event::Mouse(MouseEvent {
            kind,
            column: 2,
            row: 1,
            modifiers: KeyModifiers::empty(),
        }),
    );
    dom.layout_dom(AREA);
}

fn key(dom: &mut TuiDom, code: KeyCode) {
    handle_scroll_key(dom, KeyEvent::new(code, KeyModifiers::empty()));
    dom.layout_dom(AREA);
}

/// §6.2 with a mandatory snap container: a directional scroll — a wheel
/// tick — goes to the next snap position in its direction (the item
/// starts 3 rows apart), never between two.
#[test]
fn a_wheel_tick_snaps_to_the_next_position_in_its_direction() {
    let (mut dom, s, _) = scroller(MANDATORY, START);
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!(top(&dom, s), 3);
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!(top(&dom, s), 6);
    wheel(&mut dom, MouseEventKind::ScrollUp);
    assert_eq!(top(&dom, s), 3);
}

/// §5.1 `proximity`: the container snaps only when a snap position is
/// near the destination — rdom's threshold is 2 cells. With items 6 rows
/// apart, three ticks stay unsnapped, the fourth (`scrollTop` 4, two rows
/// short of 6) snaps.
#[test]
fn proximity_snaps_only_within_the_threshold() {
    let (mut dom, s, _) = scroller(
        "scroll-snap-type: y proximity",
        "height: 6; scroll-snap-align: start",
    );
    for expected in [1, 2, 3, 6] {
        wheel(&mut dom, MouseEventKind::ScrollDown);
        assert_eq!(top(&dom, s), expected);
    }
}

/// The keyboard: an arrow is directional (the next position), a page
/// down goes to the position in its direction nearest its destination
/// (from 0 by a 4-row page: 3, not 6), Home / End to the ends' nearest.
#[test]
fn keys_snap() {
    let (mut dom, s, _) = scroller(MANDATORY, START);
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.set_focused(Some(s));
    key(&mut dom, KeyCode::Down);
    assert_eq!(top(&dom, s), 3);
    key(&mut dom, KeyCode::PageDown);
    assert_eq!(top(&dom, s), 6);
    key(&mut dom, KeyCode::End);
    assert_eq!(
        top(&dom, s),
        11,
        "the last item's start clamped to the range"
    );
    key(&mut dom, KeyCode::Home);
    assert_eq!(top(&dom, s), 0);
}

/// §6.2 `scroll-snap-stop: always`: a scroll may not pass the position
/// — `scrollBy(8)` from 0 would snap to 9, but stops at item 1's 3;
/// past it, the next goes on to the end (11).
#[test]
fn scroll_snap_stop_always_is_not_passed() {
    let (mut dom, s, items) = scroller(MANDATORY, START);
    dom.set_attribute(items[1], "class", "i stop").unwrap();
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ width: 10; height: 4; overflow-y: scroll; {MANDATORY} }} \
         .i {{ height: 3; {START} }} .stop {{ scroll-snap-stop: always }}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    dom.node_mut(s).scroll_by(0, 8).unwrap();
    assert_eq!(top(&dom, s), 3);
    dom.layout_dom(AREA);
    dom.node_mut(s).scroll_by(0, 8).unwrap();
    assert_eq!(top(&dom, s), 11);
}

/// A non-directional scroll — `scrollTo` — snaps to the position nearest
/// its destination (5 → 6); `center` and `end` align the item's middle
/// and end with the snapport's.
#[test]
fn a_scroll_to_snaps_to_the_nearest_position() {
    let (mut dom, s, _) = scroller(MANDATORY, START);
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(top(&dom, s), 6);
    // `center`: item 2 (rows 6–9) centred in 4 rows: 6 + (3 − 4) / 2.
    let (mut dom, s, _) = scroller(MANDATORY, "scroll-snap-align: center");
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(top(&dom, s), 5);
    // `end`: item 2's end (9) at the snapport's end: 9 − 4.
    let (mut dom, s, _) = scroller(MANDATORY, "scroll-snap-align: end");
    dom.node_mut(s).scroll_to(0, 6).unwrap();
    assert_eq!(top(&dom, s), 5);
}

/// §4: the snapport is the scrollport inset by `scroll-padding`, and an
/// item's snap area its box outset by `scroll-margin`: with
/// `scroll-padding-top: 1` item 2 snaps one row lower (5), with
/// `scroll-margin-top: 1` too.
#[test]
fn padding_and_margin_move_the_snap_positions() {
    let (mut dom, s, _) = scroller(&format!("{MANDATORY}; scroll-padding-top: 1"), START);
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(top(&dom, s), 5);
    let (mut dom, s, _) = scroller(MANDATORY, &format!("{START}; scroll-margin-top: 1"));
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(top(&dom, s), 5);
}

/// A smooth scroll goes to the snapped destination: `scroll-behavior:
/// smooth` animates `scrollTo(5)` towards 6 and settles there.
#[test]
fn a_smooth_scroll_settles_on_the_snap_position() {
    let (mut dom, s, _) = scroller(&format!("{MANDATORY}; scroll-behavior: smooth"), START);
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(crate::runtime::smooth_scroll::destination(&dom, s), (0, 6));
}

/// `scrollIntoView` aligns, then snaps (§6.1): item 3 at `start` is 9.
#[test]
fn scroll_into_view_lands_on_a_snap_position() {
    let (mut dom, s, items) = scroller(MANDATORY, "scroll-snap-align: center");
    dom.node_mut(items[3]).scroll_into_view().unwrap();
    // Start-aligned 9 is no snap position; the centred ones are 2, 5, 8,
    // 11: 8 is the nearest.
    assert_eq!(top(&dom, s), 8);
}

/// Releasing a scrollbar thumb drag snaps to the nearest position.
#[test]
fn releasing_a_thumb_drag_snaps() {
    let (mut dom, s, _) = scroller(MANDATORY, START);
    let mut router = Router::new();
    let mouse = |kind, row| {
        Event::Mouse(MouseEvent {
            kind,
            column: 9,
            row,
            modifiers: KeyModifiers::empty(),
        })
    };
    use crossterm::event::MouseButton::Left;
    router.route(&mut dom, mouse(MouseEventKind::Down(Left), 0));
    router.route(&mut dom, mouse(MouseEventKind::Drag(Left), 2));
    let dragged = top(&dom, s);
    assert!(![0, 3, 6, 9, 11].contains(&dragged), "mid-drag {dragged}");
    router.route(&mut dom, mouse(MouseEventKind::Up(Left), 2));
    let nearest = [0, 3, 6, 9, 11]
        .into_iter()
        .min_by_key(|p: &i32| (p - dragged).abs())
        .unwrap();
    assert_eq!(top(&dom, s), nearest);
}

/// §5.4: after a layout change the container re-snaps to the snap
/// target it was snapped to — item 2, at 6, moves to 9 when an item is
/// inserted before it.
#[test]
fn a_layout_change_resnaps_to_the_same_target() {
    let (mut dom, s, items) = scroller(MANDATORY, START);
    dom.node_mut(s).scroll_to(0, 6).unwrap();
    dom.layout_dom(AREA);
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ width: 10; height: 4; overflow-y: scroll; {MANDATORY} }} .i {{ height: 3; {START} }}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    assert!(super::resnap(&mut dom), "the offset moved");
    assert_eq!(top(&dom, s), 9);
    dom.layout_dom(AREA);
    assert!(!super::resnap(&mut dom), "settled");
}

/// No `scroll-snap-type`, no snapping; an item's `scroll-snap-align`
/// alone does nothing.
#[test]
fn a_box_that_is_no_snap_container_does_not_snap() {
    let (mut dom, s, _) = scroller("", START);
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!(top(&dom, s), 1);
    dom.node_mut(s).scroll_to(0, 5).unwrap();
    assert_eq!(top(&dom, s), 5);
}

/// §5.4 through the frame: the `App` lays the document out and re-snaps
/// in the same frame, so the frame paints the snapped offset.
#[test]
fn the_frame_resnaps_after_layout() {
    use crate::render::{Terminal, TestBackend};
    use crate::runtime::app::App;
    let (dom, s, items) = scroller(MANDATORY, START);
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ width: 10; height: 4; overflow-y: scroll; {MANDATORY} }} .i {{ height: 3; {START} }}"
    ))
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(12, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app.dom_mut().node_mut(s).scroll_to(0, 6).unwrap();
    app.advance(16).unwrap();
    let dom = app.dom_mut();
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    app.advance(16).unwrap();
    assert_eq!(top(app.dom(), s), 9);
}
