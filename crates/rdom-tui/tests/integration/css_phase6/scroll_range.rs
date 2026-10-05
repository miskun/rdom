//! C6G-SCROLL-API — `TuiAccessors::scroll_range`: the legal `scrollLeft`
//! / `scrollTop` values of a box (CSSOM View §4), measured from its
//! scrolling area origin — which a reversed flex axis moves to the right
//! or bottom edge (CSS Flexbox §5.1 / §5.2), making the range negative.

use super::{el, lay_out};
use rdom_tui::accessors::ScrollRange;
use rdom_tui::{TuiAccessors, TuiDom};

fn scroller(dom: &mut TuiDom, items: usize) -> rdom_tui::NodeId {
    let root = dom.root();
    let s = el(dom, root, "div", "s");
    for _ in 0..items {
        el(dom, s, "div", "i");
    }
    s
}

/// CSSOM View §4: from an origin at the left / top edge the offsets run
/// `0 ..= overflow` — here 6 columns past a 4-wide scrollport.
#[test]
fn a_box_scrolls_from_zero() {
    let mut dom = TuiDom::new();
    let s = scroller(&mut dom, 1);
    lay_out(
        &mut dom,
        ".s { width: 4; height: 2; overflow: hidden } .i { width: 10; height: 1 }",
        20,
        5,
    );
    let range = dom.node(s).scroll_range().expect("an element");
    assert_eq!(range, ScrollRange::new(0..=6, 0..=0));
}

/// CSS Flexbox §5.1: a `row-reverse` container's main-start is its right
/// edge, the scrolling area origin, so `scrollLeft` runs `-6 ..= 0`.
#[test]
fn a_row_reverse_container_scrolls_left_negatively() {
    let mut dom = TuiDom::new();
    let s = scroller(&mut dom, 1);
    lay_out(
        &mut dom,
        ".s { display: flex; flex-direction: row-reverse; width: 4; height: 2; overflow: hidden } \
         .i { width: 10; height: 1; flex-shrink: 0 }",
        20,
        5,
    );
    assert_eq!(dom.node(s).scroll_range().unwrap().x(), -6..=0);
}

/// CSS Flexbox §5.2: `wrap-reverse` swaps a row's cross-start and
/// cross-end, so its lines start at the bottom and `scrollTop` runs
/// `-2 ..= 0` for 4 one-row lines in 2 rows.
#[test]
fn a_wrap_reverse_row_scrolls_up_negatively() {
    let mut dom = TuiDom::new();
    let s = scroller(&mut dom, 4);
    lay_out(
        &mut dom,
        ".s { display: flex; flex-wrap: wrap-reverse; width: 4; height: 2; overflow: hidden } \
         .i { width: 4; height: 1 }",
        20,
        5,
    );
    assert_eq!(dom.node(s).scroll_range().unwrap().y(), -2..=0);
}

/// A non-element has no scroll range.
#[test]
fn a_text_node_has_no_scroll_range() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let t = dom.create_text_node("x");
    dom.append_child(root, t).unwrap();
    lay_out(&mut dom, "", 4, 1);
    assert_eq!(dom.node(t).scroll_range(), None);
}
