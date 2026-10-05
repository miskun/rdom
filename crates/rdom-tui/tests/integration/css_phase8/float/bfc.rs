//! Block formatting context roots and floats (CSS 2.1 §9.4.1, §9.5,
//! §10.6.7), floats in `display: contents`, flex items, anonymous block
//! boxes and the scrollable overflow.

use super::place::{boxed, text};
use crate::css_phase8::{el, lay_out, paint, rect, rows};
use rdom_tui::prelude::*;

/// A `.w` block in `.c` holding a 3-row left float and one word: the
/// wrapper's height, styled `w`.
fn wrapper_height(w: &str) -> u16 {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let wrap = el(&mut dom, c, "div", "w");
    boxed(&mut dom, wrap, "f", "F");
    text(&mut dom, wrap, "word");
    lay_out(
        &mut dom,
        &format!(".c {{ width: 10 }} .f {{ float: left; width: 2; height: 3 }} .w {{ {w} }}"),
        10,
        8,
    );
    rect(&dom, wrap).height
}

/// §10.6.7: "In addition, if the element has any floating descendants
/// whose bottom margin edge is below the element's bottom content edge,
/// then the height is increased to include those edges" — for a block
/// formatting context root (`flow-root`, `overflow: hidden`); a block
/// that is none is as tall as its line (§10.6.3: floats are not in flow)
/// and the float overflows it.
#[test]
fn a_block_formatting_context_root_contains_its_floats() {
    assert_eq!(wrapper_height(""), 1);
    assert_eq!(wrapper_height("display: flow-root"), 3);
    assert_eq!(wrapper_height("overflow: hidden"), 3);
}

/// §9.5: "The border box of ... an element in the normal flow that
/// establishes a new block formatting context ... must not overlap the
/// margin box of any floats in the same block formatting context" — an
/// `auto`-width one narrows beside the float.
#[test]
fn a_formatting_context_root_goes_beside_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "f", "F");
    let b = boxed(&mut dom, c, "b", "abc");
    lay_out(
        &mut dom,
        ".c { width: 10 } .f { float: left; width: 3; height: 2 } \
         .b { display: flow-root; margin-left: 1 }",
        10,
        4,
    );
    let r = rect(&dom, b);
    assert_eq!((r.x, r.y, r.width), (4, 0, 6));
}

/// §9.5 with a declared width that does not fit beside the float: the
/// root goes below it.
#[test]
fn a_formatting_context_root_too_wide_goes_below_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "f", "F");
    let b = boxed(&mut dom, c, "b", "abc");
    lay_out(
        &mut dom,
        ".c { width: 10 } .f { float: left; width: 3; height: 2 } \
         .b { overflow: hidden; width: 8 }",
        10,
        4,
    );
    let r = rect(&dom, b);
    assert_eq!((r.x, r.y, r.width), (0, 2, 8));
}

/// A float inside a nested block formatting context root stays there:
/// the outer paragraph's lines are not shortened by it (§9.5: only floats
/// "in the same block formatting context").
#[test]
fn a_float_in_an_inner_context_does_not_reach_out() {
    let got = {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "div", "c");
        let inner = el(&mut dom, c, "div", "in");
        boxed(&mut dom, inner, "f", "FF");
        boxed(&mut dom, c, "p", "aa bb");
        let css = ".c { width: 10 } .in { display: flow-root; height: 1 } \
                   .f { float: left; width: 2; height: 2 }";
        let buf = paint(&mut dom, css, 10, 2);
        rows(&buf, 10, 2)
    };
    // The inner root's declared height is 1, so its float overflows into
    // row 1 — under the paragraph, whose text starts at column 0.
    assert_eq!(got, ["FF        ", "aa bb     "]);
}

/// CSS Display 3 §2.5: a float inside a `display: contents` element is a
/// float of the contents element's parent's flow.
#[test]
fn a_float_in_display_contents_floats_in_the_parent_flow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let k = el(&mut dom, c, "div", "k");
    boxed(&mut dom, k, "f", "XY");
    text(&mut dom, c, "aa bb");
    let buf = paint(
        &mut dom,
        ".c { width: 10 } .k { display: contents } .f { float: right; width: 2; height: 1 }",
        10,
        1,
    );
    assert_eq!(rows(&buf, 10, 1), ["aa bb   XY"]);
}

/// CSS Flexbox §4 ("float and clear do not create floating or clearance
/// of flex item, and do not take it out-of-flow") — and CSS Grid 2 §6.1
/// the same: a flex item with `float` is laid out as an item.
#[test]
fn a_flex_item_does_not_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let a = boxed(&mut dom, c, "a", "A");
    let f = boxed(&mut dom, c, "f", "F");
    lay_out(
        &mut dom,
        ".c { display: flex; width: 10 } .a { width: 2 } .f { float: right; width: 3 }",
        10,
        2,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, f).x), (0, 2));
}

/// A float beside a block child's text in mixed content: the text run's
/// anonymous block box (CSS 2.1 §9.2.1.1) flows around it.
#[test]
fn an_anonymous_block_boxs_lines_flow_around_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "p", "top");
    boxed(&mut dom, c, "f", "XY");
    text(&mut dom, c, "aa bb cc");
    let buf = paint(
        &mut dom,
        ".c { width: 8 } .f { float: left; width: 2; height: 2 }",
        8,
        3,
    );
    assert_eq!(rows(&buf, 8, 3), ["top     ", "XYaa bb ", "  cc    "]);
}

/// CSS Overflow 3 §2.2: a float is part of its scroll container's
/// scrollable overflow — its border box counts like any descendant's.
#[test]
fn a_float_counts_in_the_scrollable_overflow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "f", "");
    text(&mut dom, c, "ab");
    lay_out(
        &mut dom,
        ".c { width: 6; height: 2; overflow: hidden } \
         .f { float: right; width: 2; height: 5 }",
        10,
        4,
    );
    assert_eq!(dom.node(c).scroll_height(), Some(5));
}

/// A box that lays its children out again — here a block that is no
/// scroll container dropping a stale scroll offset (CSS Overflow 3 §3.1:
/// it has none), which re-lays its content — places its floats once: the
/// float is where one layout puts it, not beside a ghost of itself.
#[test]
fn laying_a_block_out_again_places_its_floats_once() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let p = el(&mut dom, c, "div", "p");
    let f = boxed(&mut dom, p, "f", "F");
    text(&mut dom, p, "ab");
    let css = ".c { width: 12 } .f { float: left; width: 3; height: 1 }";
    lay_out(&mut dom, css, 12, 2);
    dom.node_mut(p).ext_mut().unwrap().scroll_x = -5;
    dom.layout_dom(rdom_tui::render::Rect::new(0, 0, 12, 2));
    assert_eq!((rect(&dom, f).x, dom.node(p).scroll_left()), (0, Some(0)));
}
