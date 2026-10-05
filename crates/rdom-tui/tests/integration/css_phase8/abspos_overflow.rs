//! C8-ABSPOS-OVERFLOW — an absolutely positioned box in its scroll
//! container's scrollable overflow (CSS Overflow 3 §2.2): the area is
//! the union of the scroll container's padding box, its line boxes, "the
//! border boxes of all boxes for which it is the containing block" and
//! the scrollable overflow of those boxes — so a positioned box placed
//! past the in-flow content can be scrolled to.

use super::{el, lay_out, rect};
use rdom_tui::prelude::*;
use rdom_tui::render::Rect;

/// A 6 × 2 `.port` holding a 1-row in-flow `.row` and an absolutely
/// positioned 2 × 1 `.abs` at `top: 5; left: 8` (styled further by
/// `extra`), laid out in a 12 × 8 viewport. Returns the dom and the
/// port, row and abs ids.
fn port(extra: &str) -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let row = el(&mut dom, port, "div", "row");
    let abs = el(&mut dom, port, "div", "abs");
    lay_out(
        &mut dom,
        &format!(
            ".port {{ width: 6; height: 2; overflow: hidden; position: relative }} \
             .row {{ height: 1 }} \
             .abs {{ position: absolute; top: 5; left: 8; width: 2; height: 1 }} {extra}"
        ),
        12,
        8,
    );
    (dom, port, row, abs)
}

fn extent(dom: &TuiDom, id: NodeId) -> (Option<i32>, Option<i32>) {
    let node = dom.node(id);
    (node.scroll_width(), node.scroll_height())
}

/// §2.2: the positioned scroll container is the box's containing block
/// (CSS 2.1 §10.1), so the box's border box — 8 + 2 cells across, 5 + 1
/// rows down — is part of its scrollable overflow.
#[test]
fn an_absolutely_positioned_box_widens_and_deepens_the_overflow() {
    let (dom, port, _, _) = port("");
    assert_eq!(extent(&dom, port), (Some(10), Some(6)));
}

/// The overflow is scrollable: `scrollTop` reaches the box, the next
/// layout keeps the offset, and the box scrolls with the content
/// (C8-CB-COMPLETE: a scroll container's containing block is its
/// scrolled content).
#[test]
fn the_positioned_box_can_be_scrolled_to() {
    let (mut dom, port, row, abs) = port("");
    dom.node_mut(port).set_scroll_top(4).unwrap();
    assert_eq!(dom.node(port).scroll_top(), Some(4));
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    assert_eq!(dom.node(port).scroll_top(), Some(4));
    assert_eq!(rect(&dom, abs).y, 1, "5 rows down, scrolled by 4");
    assert_eq!(rect(&dom, row).y, -4);
}

/// §2.2 through a positioned descendant: the box's containing block is
/// `.row` (`position: relative`, no clipping), whose scrollable overflow
/// is part of the port's.
#[test]
fn a_box_contained_by_a_descendant_counts() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let row = el(&mut dom, port, "div", "row");
    el(&mut dom, row, "div", "abs");
    lay_out(
        &mut dom,
        ".port { width: 6; height: 2; overflow: hidden } \
         .row { height: 1; position: relative } \
         .abs { position: absolute; top: 3; left: 7; width: 2; height: 1 }",
        12,
        8,
    );
    assert_eq!(extent(&dom, port), (Some(9), Some(4)));
}

/// A box whose containing block is outside the scroll container is not
/// in its overflow (§2.2 counts the boxes it is the containing block
/// of): the port is static, the positioned `.wrap` around it contains
/// the box.
#[test]
fn a_box_contained_above_the_scroll_container_does_not_count() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let port = el(&mut dom, wrap, "div", "port");
    el(&mut dom, port, "div", "row");
    el(&mut dom, port, "div", "abs");
    lay_out(
        &mut dom,
        ".wrap { position: relative } \
         .port { width: 6; height: 2; overflow: hidden } .row { height: 1 } \
         .abs { position: absolute; top: 5; left: 8; width: 2; height: 1 }",
        12,
        8,
    );
    assert_eq!(extent(&dom, port), (Some(6), Some(1)));
}

/// A `fixed` box's containing block is the viewport (CSS Position 3
/// §2.1): never in a scroll container's overflow.
#[test]
fn a_fixed_box_does_not_count() {
    let (dom, port, _, _) = port(".abs { position: fixed }");
    assert_eq!(extent(&dom, port), (Some(6), Some(1)));
}

/// CSS Overflow 3 §3.1: an `auto` axis shows its scrollbar when the box
/// overflows — here only through the positioned box, so the vertical
/// bar's gutter takes the content box's last column (6 → 5).
#[test]
fn positioned_overflow_shows_an_auto_scrollbar() {
    let (dom, port, _, _) = port(".port { overflow: auto }");
    let content = dom.node(port).ext().unwrap().content_layout;
    assert_eq!((content.width, content.height), (5, 1), "both bars shown");
}

/// The clamp sees the positioned box: once it is gone the offset that
/// reached it is out of range and snaps back, the content moving with it.
#[test]
fn removing_the_box_clamps_the_offset_back() {
    let (mut dom, port, row, abs) = port("");
    dom.node_mut(port).set_scroll_top(4).unwrap();
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    dom.remove_child(port, abs).unwrap();
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    assert_eq!(dom.node(port).scroll_top(), Some(0));
    assert_eq!(rect(&dom, row).y, 0);
}

/// The part of a box before the scroll origin is unreachable (CSS
/// Overflow 3 §2.2, CSSOM View §4): a box at `left: -4` in an `ltr` port
/// adds nothing to its left, and its right edge is inside the content.
#[test]
fn a_box_before_the_scroll_origin_adds_nothing() {
    let (dom, port, _, _) = port(".abs { top: 0; left: -4 }");
    assert_eq!(extent(&dom, port), (Some(6), Some(1)));
}

/// A box is clipped by the `overflow: clip` boxes it is contained in
/// (CSS 2.1 §11.1.1): `.row` contains it and clips at its own edges, so
/// the box three rows below adds nothing to the port.
#[test]
fn a_clipping_containing_block_cuts_the_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let row = el(&mut dom, port, "div", "row");
    el(&mut dom, row, "div", "abs");
    lay_out(
        &mut dom,
        ".port { width: 6; height: 2; overflow: hidden } \
         .row { height: 1; position: relative; overflow: clip } \
         .abs { position: absolute; top: 3; left: 2; width: 2; height: 1 }",
        12,
        8,
    );
    assert_eq!(extent(&dom, port), (Some(6), Some(1)));
}
