//! Float placement and line-box shortening (CSS 2.1 §9.5, §9.5.1).
//! Every sheet styles a `.c` block container (a child of the root, so a
//! flex item of rdom's viewport column and a block formatting context
//! root) holding the floats and the content.

use crate::css_phase8::{el, paint, rect, rows};
use rdom_tui::prelude::*;

/// `.c` holding the children `build` adds, painted in `w` × `h`.
fn painted(css: &str, w: u16, h: u16, build: impl FnOnce(&mut TuiDom, NodeId)) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    build(&mut dom, c);
    let buf = paint(&mut dom, css, w, h);
    rows(&buf, w, h)
}

/// A `div.<class>` holding `text`, appended to `parent`.
pub(super) fn boxed(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let id = el(dom, parent, "div", class);
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    id
}

pub(super) fn text(dom: &mut TuiDom, parent: NodeId, text: &str) {
    let t = dom.create_text_node(text);
    dom.append_child(parent, t).unwrap();
}

const F: &str = ".f { float: left; width: 3; height: 2 } .c { width: 10 }";

/// §9.5: "a float is a box that is shifted to the left or right on the
/// current line ... content may flow along its side"; the line boxes
/// beside it are shortened. The float takes columns 0–2 of rows 0–1, the
/// text the 7 columns right of it there, then the full width.
#[test]
fn text_flows_beside_a_left_float() {
    let got = painted(F, 10, 3, |dom, c| {
        boxed(dom, c, "f", "XYZ");
        text(dom, c, "aa bb cc dd ee ff");
    });
    assert_eq!(got, ["XYZaa bb  ", "   cc dd  ", "ee ff     "]);
}

/// §9.5.1 rule 1 and rule 9: a right float's right outer edge is at the
/// containing block's right edge; the lines end left of it.
#[test]
fn text_flows_beside_a_right_float() {
    let css = ".f { float: right; width: 3; height: 2 } .c { width: 10 }";
    let got = painted(css, 10, 3, |dom, c| {
        boxed(dom, c, "f", "XYZ");
        text(dom, c, "aa bb cc dd ee ff");
    });
    assert_eq!(got, ["aa bb  XYZ", "cc dd     ", "ee ff     "]);
}

/// §9.5.1 rule 2: a later left float sits right of an earlier one; rule
/// 8: as high as possible.
#[test]
fn left_floats_stack_side_by_side() {
    let css = ".f { float: left; width: 3; height: 1 } .c { width: 10 }";
    let mut ids = Vec::new();
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    for t in ["AAA", "BBB", "CCC", "DDD"] {
        ids.push(boxed(&mut dom, c, "f", t));
    }
    crate::css_phase8::lay_out(&mut dom, css, 10, 3);
    let xy: Vec<(i32, i32)> = ids
        .iter()
        .map(|&i| (rect(&dom, i).x, rect(&dom, i).y))
        .collect();
    // The fourth does not fit beside the first three (9 + 3 > 10): rule
    // 2 puts its top below them.
    assert_eq!(xy, [(0, 0), (3, 0), (6, 0), (0, 1)]);
}

/// §9.5.1 rule 3 / rule 7: a left float wider than the space left of a
/// right float goes below it.
#[test]
fn a_float_that_does_not_fit_beside_another_moves_down() {
    let css = ".r { float: right; width: 6; height: 2 } .l { float: left; width: 5; height: 1 } \
               .c { width: 10 }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let r = boxed(&mut dom, c, "r", "");
    let l = boxed(&mut dom, c, "l", "");
    crate::css_phase8::lay_out(&mut dom, css, 10, 4);
    assert_eq!((rect(&dom, r).x, rect(&dom, r).y), (4, 0));
    assert_eq!((rect(&dom, l).x, rect(&dom, l).y), (0, 2));
}

/// §9.5.1 rule 6 and Chromium / Gecko: a float met in the middle of a
/// line that has room for it goes on that line, at its start — the text
/// already on the line moves past it.
#[test]
fn a_float_in_inline_content_joins_the_current_line() {
    let css = ".f { float: left; width: 2; height: 1 } .c { width: 10 }";
    let got = painted(css, 10, 2, |dom, c| {
        text(dom, c, "aa ");
        let s = el(dom, c, "span", "f");
        text(dom, s, "XY");
        text(dom, c, "bb cc");
    });
    assert_eq!(got, ["XYaa bb cc", "          "]);
}

/// A float met on a line too full for it goes at the top of the next
/// line (rule 6: not above the line holding earlier content).
#[test]
fn a_float_that_does_not_fit_on_the_line_goes_below_it() {
    let css = ".f { float: left; width: 4; height: 1 } .c { width: 10 }";
    let got = painted(css, 10, 3, |dom, c| {
        text(dom, c, "aaaa bbbb ");
        let s = el(dom, c, "span", "f");
        text(dom, s, "WXYZ");
        text(dom, c, "cc dd");
    });
    assert_eq!(got, ["aaaa bbbb ", "WXYZcc dd ", "          "]);
}

/// §9.5: "If a shortened line box is too small to contain any content,
/// then the line box is shifted downward ... until either some content
/// fits or there are no more floats present."
#[test]
fn a_word_too_wide_beside_a_float_moves_below_it() {
    let css = ".f { float: left; width: 6; height: 1 } .c { width: 10 }";
    let got = painted(css, 10, 2, |dom, c| {
        boxed(dom, c, "f", "XXXXXX");
        text(dom, c, "abcdefgh");
    });
    assert_eq!(got, ["XXXXXX    ", "abcdefgh  "]);
}

/// CSS Logical 1 §2.3: `inline-start` is the right side in an `rtl`
/// containing block; the lines start at the float's left edge.
#[test]
fn inline_start_floats_right_under_rtl() {
    let css = ".f { float: inline-start; width: 3; height: 1 } .c { width: 10; direction: rtl }";
    let got = painted(css, 10, 2, |dom, c| {
        boxed(dom, c, "f", "XYZ");
        text(dom, c, "aa bb");
    });
    assert_eq!(got, ["  aa bbXYZ", "          "]);
}

/// A float's line boxes in a block box between blocks: the paragraph
/// after it flows around it (§9.5: "line boxes ... next to the float are
/// shortened"), the block before it is unaffected.
#[test]
fn a_float_between_blocks_shortens_the_next_paragraphs_lines() {
    let css = ".f { float: left; width: 3; height: 2 } .c { width: 10 }";
    let got = painted(css, 10, 4, |dom, c| {
        boxed(dom, c, "p", "first");
        boxed(dom, c, "f", "XYZ");
        boxed(dom, c, "p", "aa bb cc dd ee");
    });
    assert_eq!(
        got,
        ["first     ", "XYZaa bb  ", "   cc dd  ", "ee        "]
    );
}

/// The float's text is painted inside its own box, and the paragraph's
/// background spans under the float (it is a block box, which floats do
/// not move: §9.5 "the current and subsequent line boxes ... are
/// shortened", the block boxes are not).
#[test]
fn a_block_box_is_not_moved_by_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "f", "XYZ");
    let p = boxed(&mut dom, c, "p", "aa bb");
    crate::css_phase8::lay_out(&mut dom, F, 10, 3);
    assert_eq!((rect(&dom, p).x, rect(&dom, p).width), (0, 10));
}
