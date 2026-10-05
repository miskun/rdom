//! C6-MARGIN-SIDES — `margin-*` and `padding-*` are independent
//! longhands (CSS Box 3 §3.2 / §4.2: the shorthand sets the four), each
//! cascading alone with its own importance (CSS Cascade 4 §6.4).

use super::{el, lay_out};
use rdom_tui::layout::{MarginValue, PaddingValue};
use rdom_tui::{TuiDom, TuiNodeExt};

/// `div.a.b` (under `div.p`) laid out under `css`.
fn one(css: &str) -> rdom_tui::ComputedStyle {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let b = el(&mut dom, p, "div", "a b");
    lay_out(&mut dom, css, 30, 10);
    dom.node(b).computed().unwrap().clone()
}

/// CSS Cascade 4 §6.4 with CSS Logical 1 §4: the important
/// `margin-inline-start` is the left margin's declaration only; the
/// normal `margin-right` beside it loses to the more specific rule's.
/// A browser gives a right margin of 5.
#[test]
fn an_important_logical_side_leaves_the_other_sides_normal() {
    let c = one(".a { margin-inline-start: 1 !important; margin-right: 2 } \
                 .a.b { margin-right: 5; margin-left: 9 }");
    assert_eq!(c.margin.left, MarginValue::Cells(1));
    assert_eq!(c.margin.right, MarginValue::Cells(5));

    let c = one(
        ".a { padding-inline-start: 1 !important; padding-right: 2 } \
                 .a.b { padding-right: 5; padding-left: 9 }",
    );
    assert_eq!(c.padding.left, PaddingValue::Cells(1));
    assert_eq!(c.padding.right, PaddingValue::Cells(5));
}

/// CSS Cascade 4 §6: each longhand is cascaded on its own, so a later
/// rule's `margin-left` leaves an earlier rule's `margin-top` in place.
#[test]
fn a_side_longhand_cascades_alone() {
    let c = one(".a { margin-top: 1; padding: 2 } .a.b { margin-left: 3; padding-left: 4 }");
    assert_eq!(c.margin.top, MarginValue::Cells(1));
    assert_eq!(c.margin.left, MarginValue::Cells(3));
    assert_eq!(c.padding.top, PaddingValue::Cells(2));
    assert_eq!(c.padding.left, PaddingValue::Cells(4));
}

/// CSS Cascade 4 §7.2: `inherit` on one longhand takes that side of the
/// parent's value, and leaves the element's other sides alone.
#[test]
fn inherit_on_one_side_takes_that_side() {
    let c = one(".p { padding: 1 2 3 4; margin: 1 2 3 4 } \
                 .a { padding: 7; margin: 7 } \
                 .a.b { padding-top: inherit; margin-bottom: inherit }");
    assert_eq!(c.padding.top, PaddingValue::Cells(1));
    assert_eq!(c.padding.left, PaddingValue::Cells(7));
    assert_eq!(c.margin.bottom, MarginValue::Cells(3));
    assert_eq!(c.margin.top, MarginValue::Cells(7));
}

/// CSS Cascade 4 §6.4: an important side longhand beats a later
/// normal shorthand on that side only.
#[test]
fn an_important_side_beats_a_later_shorthand_on_that_side_only() {
    let c = one(".a { margin-top: 4 !important } .a.b { margin: 1 }");
    assert_eq!(c.margin.top, MarginValue::Cells(4));
    assert_eq!(c.margin.bottom, MarginValue::Cells(1));
}
