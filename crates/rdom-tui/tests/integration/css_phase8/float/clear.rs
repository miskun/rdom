//! Clearance (CSS 2.1 §9.5.2).

use super::place::{boxed, text};
use crate::css_phase8::{el, lay_out, rect};
use rdom_tui::prelude::*;

/// `.c` (10 wide) holding a left float `.l` (3 × 2), a right float `.r`
/// (3 × 4) and a block `.b` styled `b`: the block's border box.
fn cleared(b: &str) -> rdom_tui::layout::LayoutRect {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "l", "L");
    boxed(&mut dom, c, "r", "R");
    let block = boxed(&mut dom, c, "b", "text");
    lay_out(
        &mut dom,
        &format!(
            ".c {{ width: 10 }} .l {{ float: left; width: 3; height: 2 }} \
             .r {{ float: right; width: 3; height: 4 }} .b {{ {b} }}"
        ),
        10,
        8,
    );
    rect(&dom, block)
}

/// §9.5.2: "the top border edge of the box [is] below the bottom outer
/// edge of any left-floating boxes" (`left`), right ones (`right`), both
/// (`both`); `none` leaves it at the top.
#[test]
fn clear_moves_a_block_below_the_named_floats() {
    assert_eq!(cleared("").y, 0);
    assert_eq!(cleared("clear: left").y, 2);
    assert_eq!(cleared("clear: right").y, 4);
    assert_eq!(cleared("clear: both").y, 4);
}

/// CSS Logical 1 §2.3: `clear: inline-end` is `right` under `ltr`.
#[test]
fn clear_inline_end_is_the_right_side_under_ltr() {
    assert_eq!(cleared("clear: inline-end").y, 4);
    assert_eq!(cleared("clear: inline-start").y, 2);
}

/// §9.5.2: clearance is the greater of the float's bottom and the
/// hypothetical position — a top margin that already takes the box below
/// the floats is kept.
#[test]
fn clearance_never_moves_a_block_up() {
    assert_eq!(cleared("clear: left; margin-top: 3").y, 3);
}

/// §9.5.2 for floats: "the top outer edge of the float must be below the
/// bottom outer edge of all earlier left-floating boxes" it clears.
#[test]
fn a_float_with_clear_goes_below_the_earlier_floats() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "l", "");
    let second = boxed(&mut dom, c, "l2", "");
    lay_out(
        &mut dom,
        ".c { width: 10 } .l { float: left; width: 3; height: 2 } \
         .l2 { float: left; width: 3; height: 1; clear: left }",
        10,
        4,
    );
    assert_eq!((rect(&dom, second).x, rect(&dom, second).y), (0, 2));
}

/// `clear` on an inline-level box does nothing (CSS 2.1 §9.5.2 applies
/// to block-level elements): its text flows beside the float.
#[test]
fn clear_on_an_inline_box_does_nothing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "l", "L");
    let s = el(&mut dom, c, "span", "s");
    text(&mut dom, s, "ab");
    let buf = crate::css_phase8::paint(
        &mut dom,
        ".c { width: 10 } .l { float: left; width: 3; height: 2 } .s { clear: left }",
        10,
        2,
    );
    assert_eq!(crate::css_phase8::rows(&buf, 10, 1)[0], "L  ab     ");
}

/// C8G-CLEARANCE-COLLAPSE — CSS 2.1 §8.3.1: "The top margin of a box
/// collapses with its first in-flow child's top margin if the element has
/// no top border, no top padding, and the child has no clearance"; §9.5.2
/// decides clearance from the child's hypothetical position. The float is
/// out of flow, so the cleared div is the parent's first in-flow child:
/// its hypothetical top (its margin collapsed through to the parent's
/// top) is above the float's bottom, so it has clearance — its `margin-top:
/// 2` does not escape, the parent and the float stay at row 0, and `x` is
/// below the float at row 3 (the greater of its margin's 2 and the
/// float's bottom).
#[test]
fn clearance_stops_the_parent_and_first_child_margins_collapsing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let parent = el(&mut dom, body, "div", "p");
    let f = boxed(&mut dom, parent, "f", "F");
    let x = boxed(&mut dom, parent, "x", "x");
    lay_out(
        &mut dom,
        ".f { float: left; width: 3; height: 3 } .x { clear: left; margin-top: 2 }",
        10,
        8,
    );
    assert_eq!(
        (rect(&dom, parent).y, rect(&dom, f).y, rect(&dom, x).y),
        (0, 0, 3)
    );
}

/// Without `clear`, the child has no clearance and its margin escapes as
/// before: the parent (and the float in it) move down 2.
#[test]
fn without_clear_the_first_childs_margin_still_collapses_through() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let parent = el(&mut dom, body, "div", "p");
    let f = boxed(&mut dom, parent, "f", "F");
    let x = boxed(&mut dom, parent, "x", "x");
    lay_out(
        &mut dom,
        ".f { float: left; width: 3; height: 3 } .x { margin-top: 2 }",
        10,
        8,
    );
    assert_eq!(
        (rect(&dom, parent).y, rect(&dom, f).y, rect(&dom, x).y),
        (2, 2, 2)
    );
}

/// C9G-CLEARANCE-COST — the same with the float a floated `::before`
/// (CSS 2.1 §12.1: the host's first child box): the cleared div is still
/// the first in-flow child, has clearance, and stops the collapse.
#[test]
fn a_floated_before_gives_the_first_child_clearance_too() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let parent = el(&mut dom, body, "div", "p");
    let x = boxed(&mut dom, parent, "x", "x");
    lay_out(
        &mut dom,
        ".p::before { content: ''; float: left; width: 3; height: 3 } \
         .x { clear: left; margin-top: 2 }",
        10,
        8,
    );
    assert_eq!((rect(&dom, parent).y, rect(&dom, x).y), (0, 3));
}

/// C9G-CLEARANCE-COST — without `clear`, a floated `::before` (out of flow)
/// does not stop the first child's margin collapsing through the parent:
/// the parent moves down 2, as with a floated element (the margin was
/// lost: the chain stopped at the pseudo-element while the flow still
/// suppressed the child's margin inside the parent).
#[test]
fn a_floated_before_does_not_stop_the_collapse() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let parent = el(&mut dom, body, "div", "p");
    let x = boxed(&mut dom, parent, "x", "x");
    lay_out(
        &mut dom,
        ".p::before { content: ''; float: left; width: 3; height: 3 } .x { margin-top: 2 }",
        10,
        8,
    );
    assert_eq!((rect(&dom, parent).y, rect(&dom, x).y), (2, 2));
}
