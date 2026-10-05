//! C8G-SNAP-TALL — CSS Scroll Snap 1 §6.2.3 "Snapping Boxes that Overflow
//! the Scrollport": "If the snap area is larger than the snapport in a
//! particular axis, and the snap area overflows the snapport … any scroll
//! position in which the snap area covers the snapport … is a valid snap
//! position in that axis." Inside a card taller than the list, scrolling
//! is free; between cards it snaps.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::accessors::TuiAccessors;
use crate::render::{LayoutExt, Rect};
use crate::runtime::router::Router;
use crate::runtime::scrollbar::handle_scroll_key;
use crate::style::CascadeExt;

const AREA: Rect = Rect::new(0, 0, 12, 12);

/// `.list` — 10 × 10, `y mandatory`, four 30-row `.card`s aligned
/// `start` (positions 0, 30, 60, 90 — the last clamped to the range's
/// end, 110), focused.
fn list() -> (TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let l = dom.create_element("div");
    dom.set_attribute(l, "class", "list").unwrap();
    dom.set_attribute(l, "tabindex", "0").unwrap();
    dom.append_child(root, l).unwrap();
    for _ in 0..4 {
        let c = dom.create_element("div");
        dom.set_attribute(c, "class", "card").unwrap();
        dom.append_child(l, c).unwrap();
    }
    let sheet = rdom_css::from_css_strict(
        ".list { width: 10; height: 10; overflow-y: auto; scroll-snap-type: y mandatory } \
         .card { height: 30; scroll-snap-align: start }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    dom.set_focused(Some(l));
    (dom, l)
}

fn top(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).scroll_top().unwrap()
}

fn page_down(dom: &mut TuiDom) {
    handle_scroll_key(dom, KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
    dom.layout_dom(AREA);
}

/// PageDown pages through the first card's rows 10–29 (each destination
/// leaves the card covering the list), then snaps to the second card.
#[test]
fn page_down_reveals_the_middle_of_a_tall_card() {
    let (mut dom, l) = list();
    let mut seen = Vec::new();
    for _ in 0..3 {
        page_down(&mut dom);
        seen.push(top(&dom, l));
    }
    assert_eq!(seen, [10, 20, 30]);
}

/// The wheel moves a row at a time inside the card, and snaps to the
/// next card's start past its end.
#[test]
fn the_wheel_scrolls_freely_inside_a_tall_card() {
    let (mut dom, l) = list();
    let mut router = Router::new();
    let mut tick = |dom: &mut TuiDom| {
        router.route(
            dom,
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 2,
                row: 2,
                modifiers: KeyModifiers::empty(),
            }),
        );
        dom.layout_dom(AREA);
    };
    tick(&mut dom);
    assert_eq!(top(&dom, l), 1);
    for _ in 0..19 {
        tick(&mut dom);
    }
    assert_eq!(top(&dom, l), 20, "the card's last rows");
    tick(&mut dom);
    assert_eq!(top(&dom, l), 30, "the next card");
}

/// A scroll to a destination (`scrollTo`, a released thumb drag) inside
/// the covering range rests there.
#[test]
fn scroll_to_inside_a_tall_card_rests_there() {
    use crate::accessors::TuiAccessorsMut;
    let (mut dom, l) = list();
    dom.node_mut(l).scroll_to(0, 15).unwrap();
    assert_eq!(top(&dom, l), 15);
}

/// §5.4 inside a tall box: a container 15 rows into the first card stays
/// 15 rows into it when a 2-row box is inserted before the cards.
#[test]
fn a_resnap_keeps_the_place_inside_a_tall_card() {
    use crate::accessors::TuiAccessorsMut;
    let (mut dom, l) = list();
    dom.node_mut(l).scroll_to(0, 15).unwrap();
    dom.layout_dom(AREA);
    let first = dom.node(l).first_child().unwrap().id();
    let head = dom.create_element("div");
    dom.insert_before(l, head, Some(first)).unwrap();
    dom.set_attribute(head, "class", "head").unwrap();
    dom.cascade(
        &rdom_css::from_css_strict(
            ".list { width: 10; height: 10; overflow-y: auto; scroll-snap-type: y mandatory } \
             .card { height: 30; scroll-snap-align: start } .head { height: 2 }",
        )
        .unwrap(),
    );
    dom.layout_dom(AREA);
    assert!(super::resnap(&mut dom));
    assert_eq!(top(&dom, l), 17);
}
