//! C6-DIRECTION-REVERSE — vertical scrolling of a `column-reverse` flex
//! container.
//!
//! CSSOM View §4 with CSS Flexbox §5.1: the scrolling area origin of a
//! `column-reverse` box is its bottom (main-start) edge, so it starts
//! scrolled to the bottom, `scrollTop` is 0 there and negative towards
//! its top overflow — as current browsers do. Wheel, keyboard and the
//! scrollbar reach that overflow.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{LayoutExt, Rect};
use crate::runtime::hit_test::HitTestExt;
use crate::runtime::router::Router;
use crate::runtime::scrollbar::{ScrollAxis, ScrollbarPart, handle_scroll_key, hit};
use crate::style::CascadeExt;
use crate::{TuiDom, TuiNodeExt};

const AREA: Rect = Rect::new(0, 0, 12, 8);

/// `.s` — a `column-reverse` flex container 10 × 4 with `overflow-y:
/// auto`, holding two unshrinkable 3-row items: 6 rows of content in a
/// 4-row scrollport, 2 of them above its top edge.
fn scroller() -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let mut item = || {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", "i").unwrap();
        dom.append_child(s, id).unwrap();
        id
    };
    let a = item();
    let b = item();
    let sheet = rdom_css::from_css_strict(
        ".s { display: flex; flex-direction: column-reverse; overflow-y: auto; \
              width: 10; height: 4 } \
         .i { flex-shrink: 0; height: 3 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    (dom, s, a, b)
}

fn y_of(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).layout_rect().unwrap().y
}

fn scroll_top(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).scroll_top().unwrap()
}

fn wheel(kind: MouseEventKind) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: 2,
        row: 1,
        modifiers: KeyModifiers::empty(),
    })
}

/// At rest the bottom shows: the first item sits at the bottom, the
/// second overflows above; `scrollTop` is 0 and `scrollHeight` covers
/// the overflow.
#[test]
fn a_column_reverse_scroller_starts_at_its_bottom() {
    let (dom, s, a, b) = scroller();
    assert_eq!(scroll_top(&dom, s), 0);
    assert_eq!(dom.node(s).scroll_height(), Some(6));
    assert_eq!((y_of(&dom, a), y_of(&dom, b)), (1, -2));
}

/// The wheel scrolls up into the overflow, to `-(6 - 4)` and no
/// further, and back down to the origin.
#[test]
fn the_wheel_reaches_both_extents() {
    let (mut dom, s, a, b) = scroller();
    let mut router = Router::new();
    for _ in 0..10 {
        router.route(&mut dom, wheel(MouseEventKind::ScrollUp));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_top(&dom, s), -2);
    assert_eq!(y_of(&dom, b), 0);
    for _ in 0..10 {
        router.route(&mut dom, wheel(MouseEventKind::ScrollDown));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_top(&dom, s), 0);
    assert_eq!(y_of(&dom, a), 1);
}

/// ArrowUp / ArrowDown on the focused container reach each extent.
#[test]
fn the_arrow_keys_reach_both_extents() {
    let (mut dom, s, _, b) = scroller();
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.set_focused(Some(s));
    let key = |code| KeyEvent::new(code, KeyModifiers::empty());
    for _ in 0..10 {
        handle_scroll_key(&mut dom, key(KeyCode::Up));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_top(&dom, s), -2);
    assert_eq!(y_of(&dom, b), 0);
    for _ in 0..10 {
        handle_scroll_key(&mut dom, key(KeyCode::Down));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_top(&dom, s), 0);
}

/// The vertical thumb starts at the bottom of the track (the scroll
/// position is at the bottom edge) and moves up as the box scrolls up.
#[test]
fn the_thumb_starts_at_the_bottom() {
    let (mut dom, s, _, _) = scroller();
    // Track: column 9, rows 0..4; thumb 4 × 4 / 6 = 3 rows.
    let part = |dom: &TuiDom, y: u16| {
        let path = dom.hit_test_path(9, y);
        let h = hit(dom, &path, 9, y).expect("on the scrollbar");
        assert_eq!((h.element, h.axis), (s, ScrollAxis::Vertical));
        h.part
    };
    assert_eq!(part(&dom, 3), ScrollbarPart::Thumb);
    assert_eq!(part(&dom, 0), ScrollbarPart::TrackBefore);
    dom.node_mut(s).set_scroll_top(-2).unwrap();
    dom.layout_dom(AREA);
    assert_eq!(part(&dom, 0), ScrollbarPart::Thumb);
    assert_eq!(part(&dom, 3), ScrollbarPart::TrackAfter);
}
