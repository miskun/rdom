//! C5G-RTL-SCROLL — horizontal scrolling of an `rtl` scroll container.
//!
//! CSSOM View §4 (scrolling area origin) and the `scrollLeft` getter: an
//! `rtl` box's scrolling area origin is its top-right corner, so the
//! initial scroll position shows the right edge, `scrollLeft` is 0 there
//! and negative towards the left overflow (`-(scrollWidth -
//! clientWidth)` at the far left), as in every current browser. Every
//! input reaches the overflow: wheel, keyboard, scrollbar drag,
//! `scrollIntoView` and the programmatic API.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rdom_core::NodeId;

use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{LayoutExt, Rect};
use crate::runtime::hit_test::HitTestExt;
use crate::runtime::router::Router;
use crate::runtime::scrollbar::{ScrollAxis, ScrollbarPart, handle_scroll_key, hit};
use crate::style::CascadeExt;
use crate::{TuiDom, TuiNodeExt};

const AREA: Rect = Rect::new(0, 0, 12, 4);

/// `.s` — an `rtl` row flex container 10 × 3 with `overflow-x: auto`,
/// holding two unshrinkable 8-cell items: 16 cells of content in a
/// 10-cell scrollport, 6 of them past the left edge.
fn flex_scroller() -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let mut item = |class: &str| {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", class).unwrap();
        dom.append_child(s, id).unwrap();
        id
    };
    let a = item("a");
    let b = item("b");
    let sheet = rdom_css::from_css_strict(
        ".s { direction: rtl; display: flex; flex-direction: row; overflow-x: auto; \
              width: 10; height: 3 } \
         .a, .b { flex-shrink: 0; width: 8; height: 1 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    (dom, s, a, b)
}

fn x_of(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).layout_rect().unwrap().x
}

fn scroll_left(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).scroll_left().unwrap()
}

fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

/// At the initial scroll position the right edge shows: the first item
/// sits at the right, the second overflows to the left; `scrollLeft` is
/// 0 and `scrollWidth` covers the left overflow.
#[test]
fn an_rtl_scroller_starts_at_its_right_edge() {
    let (dom, s, a, b) = flex_scroller();
    assert_eq!(scroll_left(&dom, s), 0);
    assert_eq!(dom.node(s).scroll_width(), Some(16));
    assert_eq!(x_of(&dom, a), 2);
    assert_eq!(x_of(&dom, b), -6);
}

/// The programmatic API takes `scrollLeft` in `[-(16 - 10), 0]`: −6
/// brings the left overflow into view, a positive value clamps to 0.
#[test]
fn scroll_left_runs_negative_to_the_left_extent() {
    let (mut dom, s, a, b) = flex_scroller();
    dom.node_mut(s).set_scroll_left(-6).unwrap();
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), -6);
    assert_eq!((x_of(&dom, b), x_of(&dom, a)), (0, 8));
    dom.node_mut(s).set_scroll_left(-100).unwrap();
    assert_eq!(scroll_left(&dom, s), -6, "clamped at the left extent");
    dom.node_mut(s).set_scroll_left(5).unwrap();
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), 0, "clamped at the origin");
    assert_eq!(x_of(&dom, a), 2);
}

/// The horizontal scrollbar's thumb starts at the right end of the
/// track (the scroll position is at the right edge) and moves left as
/// the box scrolls left.
#[test]
fn the_thumb_starts_at_the_right() {
    let (mut dom, s, _, _) = flex_scroller();
    // Track: the bottom row, x 0..10; thumb 10 × 10 / 16 = 6 cells.
    let part = |dom: &TuiDom, x: u16| {
        let path = dom.hit_test_path(x, 2);
        let h = hit(dom, &path, x, 2).expect("on the scrollbar");
        assert_eq!((h.element, h.axis), (s, ScrollAxis::Horizontal));
        h.part
    };
    assert_eq!(part(&dom, 9), ScrollbarPart::Thumb);
    assert_eq!(part(&dom, 4), ScrollbarPart::Thumb);
    assert_eq!(part(&dom, 3), ScrollbarPart::TrackBefore);
    dom.node_mut(s).set_scroll_left(-6).unwrap();
    dom.layout_dom(AREA);
    assert_eq!(part(&dom, 0), ScrollbarPart::Thumb);
    assert_eq!(part(&dom, 9), ScrollbarPart::TrackAfter);
}

/// Wheel left scrolls towards the left overflow, to its extent and no
/// further; wheel right scrolls back to the origin.
#[test]
fn the_wheel_reaches_both_extents() {
    let (mut dom, s, a, b) = flex_scroller();
    let mut router = Router::new();
    for _ in 0..10 {
        router.route(&mut dom, mouse(MouseEventKind::ScrollLeft, 5, 0));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_left(&dom, s), -6);
    assert_eq!((x_of(&dom, b), x_of(&dom, a)), (0, 8));
    for _ in 0..10 {
        router.route(&mut dom, mouse(MouseEventKind::ScrollRight, 5, 0));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_left(&dom, s), 0);
    assert_eq!(x_of(&dom, a), 2);
}

/// ArrowLeft / ArrowRight on the focused scroll container move the
/// view left / right, to each extent.
#[test]
fn the_arrow_keys_reach_both_extents() {
    let (mut dom, s, _, b) = flex_scroller();
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.set_focused(Some(s));
    let key = |code| KeyEvent::new(code, KeyModifiers::empty());
    for _ in 0..10 {
        assert!(handle_scroll_key(&mut dom, key(KeyCode::Left)));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_left(&dom, s), -6);
    assert_eq!(x_of(&dom, b), 0);
    for _ in 0..10 {
        handle_scroll_key(&mut dom, key(KeyCode::Right));
        dom.layout_dom(AREA);
    }
    assert_eq!(scroll_left(&dom, s), 0);
}

/// Dragging the thumb from the right end to the left end of the track
/// scrolls to the left extent, and back.
#[test]
fn dragging_the_thumb_reaches_both_extents() {
    let (mut dom, s, _, b) = flex_scroller();
    let mut router = Router::new();
    router.route(
        &mut dom,
        mouse(MouseEventKind::Down(MouseButton::Left), 7, 2),
    );
    router.route(
        &mut dom,
        mouse(MouseEventKind::Drag(MouseButton::Left), 0, 2),
    );
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), -6);
    assert_eq!(x_of(&dom, b), 0);
    router.route(
        &mut dom,
        mouse(MouseEventKind::Drag(MouseButton::Left), 11, 2),
    );
    router.route(
        &mut dom,
        mouse(MouseEventKind::Up(MouseButton::Left), 11, 2),
    );
    assert_eq!(scroll_left(&dom, s), 0);
}

/// `scrollIntoView` of the overflowing item scrolls left until it is
/// in view (`inline: nearest`), and of the first item back again.
#[test]
fn scroll_into_view_reaches_the_left_overflow() {
    let (mut dom, s, a, b) = flex_scroller();
    dom.node_mut(b).scroll_into_view().unwrap();
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), -6);
    assert_eq!(x_of(&dom, b), 0);
    dom.node_mut(a).scroll_into_view().unwrap();
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), 0);
}

/// CSS 2.1 §10.3.3: an over-wide block child of an `rtl` block drops
/// its left margin, so it overflows to the left; that overflow is the
/// scrollable area left of the origin.
#[test]
fn an_over_wide_block_child_scrolls_into_view() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let w = dom.create_element("div");
    dom.set_attribute(w, "class", "w").unwrap();
    dom.append_child(s, w).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { direction: rtl; overflow-x: auto; width: 10; height: 3 } \
         .w { width: 16; height: 1 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    assert_eq!(scroll_left(&dom, s), 0);
    assert_eq!(x_of(&dom, w), -6);
    assert_eq!(dom.node(s).scroll_width(), Some(16));
    dom.node_mut(s).set_scroll_left(-6).unwrap();
    dom.layout_dom(AREA);
    assert_eq!(x_of(&dom, w), 0);
}

/// CSSOM View §5.1 with CSS Writing Modes 4 §2.1: `inline: start`
/// aligns the element's inline-start edge — its right edge in an `rtl`
/// box — with the scrollport's, and `inline: end` its left edge.
#[test]
fn scroll_into_view_inline_start_is_the_right_edge() {
    use crate::runtime::smooth_scroll::{ScrollIntoViewOptions, ScrollLogicalPosition};
    let (mut dom, s, _, b) = flex_scroller();
    let c = dom.create_element("div");
    dom.set_attribute(c, "class", "b").unwrap();
    dom.append_child(s, c).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { direction: rtl; display: flex; flex-direction: row; overflow-x: auto; \
              width: 10; height: 3 } \
         .a, .b { flex-shrink: 0; width: 8; height: 1 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    // 24 cells of content: `b` at -6..2, `c` at -14..-6.
    assert_eq!(x_of(&dom, b), -6);
    let to = |dom: &mut TuiDom, position| {
        dom.node_mut(b)
            .scroll_into_view_with(ScrollIntoViewOptions::new().inline(position))
            .unwrap();
        dom.layout_dom(AREA);
        (scroll_left(dom, s), x_of(dom, b))
    };
    assert_eq!(
        to(&mut dom, ScrollLogicalPosition::Start),
        (-8, 2),
        "right edges meet"
    );
    assert_eq!(
        to(&mut dom, ScrollLogicalPosition::End),
        (-6, 0),
        "left edges meet"
    );
}
