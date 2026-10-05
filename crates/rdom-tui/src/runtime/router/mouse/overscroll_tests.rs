//! C8-OVERSCROLL — scroll chaining for the wheel (CSS Overscroll Behavior
//! 1 §3): `auto` chains a scroll a box cannot take to its scrollable
//! ancestor; `contain` and `none` stop it there (a terminal has no
//! overscroll affordance to tell the two apart).

use crossterm::event::{KeyModifiers, MouseEvent as CtMouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::render::{LayoutExt, Rect};
use crate::runtime::router::Router;
use crate::{CascadeExt, TuiDom};

/// An `.outer` scroller (20 × 6, 40 rows of content) holding an `.inner`
/// one (10 × 4, 10 rows) at its top, `.inner` styled `inner` and
/// scrolled to its bottom and its right end (as far as the runtime's
/// clamp lets a script scroll it).
fn nested(inner: &str) -> (TuiDom, NodeId, NodeId) {
    nested_with(inner, "height: 10; width: 30")
}

/// [`nested`] with `.tall` styled `tall`.
fn nested_with(inner: &str, tall: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let el = |dom: &mut TuiDom, parent: NodeId, class: &str| {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", class).unwrap();
        dom.append_child(parent, id).unwrap();
        id
    };
    let outer = el(&mut dom, root, "outer");
    let inner_id = el(&mut dom, outer, "inner");
    el(&mut dom, inner_id, "tall");
    el(&mut dom, outer, "spacer");
    let sheet = rdom_css::from_css_strict(&format!(
        ".outer {{ width: 20; height: 6; overflow: auto }} \
         .inner {{ width: 10; height: 4; overflow: auto; {inner} }} \
         .tall {{ {tall} }} .spacer {{ height: 40; width: 40 }}"
    ))
    .expect("sheet parses");
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));
    {
        use crate::accessors::TuiAccessorsMut;
        let mut n = dom.node_mut(inner_id);
        n.set_scroll_top(i32::MAX).unwrap();
        n.set_scroll_left(i32::MAX).unwrap();
    }
    (dom, outer, inner_id)
}

fn wheel(dom: &mut TuiDom, kind: MouseEventKind) {
    let mut router = Router::new();
    router.route(
        dom,
        crossterm::event::Event::Mouse(CtMouseEvent {
            kind,
            column: 2,
            row: 1,
            modifiers: KeyModifiers::empty(),
        }),
    );
}

fn offsets(dom: &TuiDom, id: NodeId) -> (i32, i32) {
    let ext = dom.node(id).ext().unwrap();
    (ext.scroll_x, ext.scroll_y)
}

/// §3 `auto`: "the default scroll overflow behavior" — the wheel at the
/// inner box's end scrolls the outer box.
#[test]
fn auto_chains_to_the_ancestor() {
    let (mut dom, outer, inner) = nested("");
    let end = offsets(&dom, inner).1;
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!((offsets(&dom, inner).1, offsets(&dom, outer).1), (end, 1));
}

/// §3 `contain`: "no scroll chaining occurs to neighboring scrolling
/// areas"; `none` likewise.
#[test]
fn contain_and_none_stop_the_chain() {
    for value in [
        "overscroll-behavior: contain",
        "overscroll-behavior-y: none",
    ] {
        let (mut dom, outer, inner) = nested(value);
        let end = offsets(&dom, inner).1;
        wheel(&mut dom, MouseEventKind::ScrollDown);
        assert_eq!(
            (offsets(&dom, inner).1, offsets(&dom, outer).1),
            (end, 0),
            "{value}"
        );
    }
}

/// The value is per axis: `overscroll-behavior-y: contain` stops a
/// vertical wheel only; `overscroll-behavior-inline` is the `x` axis in
/// `horizontal-tb`.
#[test]
fn each_axis_has_its_own_value() {
    let (mut dom, outer, _) = nested("overscroll-behavior-y: contain");
    wheel(&mut dom, MouseEventKind::ScrollRight);
    assert_eq!(offsets(&dom, outer).0, 1, "x still chains");
    let (mut dom, outer, _) = nested("overscroll-behavior-inline: contain");
    wheel(&mut dom, MouseEventKind::ScrollRight);
    assert_eq!(offsets(&dom, outer).0, 0, "inline is x");
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!(offsets(&dom, outer).1, 1, "y still chains");
}

/// A box that can still scroll takes the wheel whatever its value: the
/// property only governs what happens at its boundary.
#[test]
fn a_box_that_can_scroll_takes_the_wheel() {
    let (mut dom, outer, inner) = nested("overscroll-behavior: contain");
    let end = offsets(&dom, inner).1;
    wheel(&mut dom, MouseEventKind::ScrollUp);
    assert_eq!(
        (offsets(&dom, inner).1, offsets(&dom, outer).1),
        (end - 1, 0)
    );
}

/// CSS Overscroll Behavior 1 §3 applies `overscroll-behavior` to every
/// scroll container, "regardless of whether those elements currently have
/// overflowing content or are user scrollable" — Chromium since 144:
/// `contain` on an `overflow: auto` box whose content fits, and on an
/// `overflow: hidden` one (a modal's backdrop), keeps the wheel from the
/// page behind.
#[test]
fn contain_stops_the_chain_at_a_box_that_cannot_scroll() {
    for inner in [
        "overflow: hidden; overscroll-behavior: contain",
        "overscroll-behavior: contain",
    ] {
        let (mut dom, outer, _) = nested_with(inner, "height: 2; width: 5");
        wheel(&mut dom, MouseEventKind::ScrollDown);
        assert_eq!(offsets(&dom, outer).1, 0, "{inner}");
    }
    // `auto` there chains, as before.
    let (mut dom, outer, _) = nested_with("overflow: hidden", "height: 2; width: 5");
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!(offsets(&dom, outer).1, 1);
}

/// CSS Scroll Snap 1 §6.2 with Overscroll Behavior §3: a mandatory snap
/// that keeps the box where it is took the tick — the box is not at its
/// boundary — so nothing chains. `.inner`'s snap positions are 0 and 3,
/// and from 3 the next tick has no position ahead: it rests at 3.
#[test]
fn a_mandatory_snap_that_holds_does_not_chain() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let el = |dom: &mut TuiDom, parent: NodeId, class: &str| {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", class).unwrap();
        dom.append_child(parent, id).unwrap();
        id
    };
    let outer = el(&mut dom, root, "outer");
    let inner = el(&mut dom, outer, "inner");
    el(&mut dom, inner, "i");
    el(&mut dom, inner, "i");
    el(&mut dom, inner, "rest");
    el(&mut dom, outer, "spacer");
    let sheet = rdom_css::from_css_strict(
        ".outer { width: 20; height: 6; overflow: auto } \
         .inner { width: 10; height: 4; overflow-y: auto; scroll-snap-type: y mandatory } \
         .i { height: 3; scroll-snap-align: start } .rest { height: 20 } \
         .spacer { height: 40 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));
    {
        use crate::accessors::TuiAccessorsMut;
        dom.node_mut(inner).scroll_to(0, 3).unwrap();
    }
    assert_eq!(offsets(&dom, inner).1, 3);
    wheel(&mut dom, MouseEventKind::ScrollDown);
    assert_eq!((offsets(&dom, inner).1, offsets(&dom, outer).1), (3, 0));
}
