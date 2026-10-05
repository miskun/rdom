//! Floats with `margin-trim` (CSS Box 4 §3), line clamping (CSS
//! Overflow 4 §4) and `text-overflow` (CSS Overflow 4 §3).

use super::place::{boxed, text};
use crate::css_phase8::{el, lay_out, paint, rect, rows};
use rdom_tui::prelude::*;

/// The border boxes' `(x, y)` of two left floats in a `.c` styled `c`.
fn two_floats(c: &str) -> [(i32, i32); 2] {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cont = el(&mut dom, root, "div", "c");
    let a = boxed(&mut dom, cont, "f", "");
    let b = boxed(&mut dom, cont, "f", "");
    lay_out(
        &mut dom,
        &format!(
            ".c {{ width: 20; {c} }} .f {{ float: left; width: 3; height: 1; \
             margin-left: 2; margin-top: 1 }}"
        ),
        20,
        4,
    );
    [a, b].map(|id| (rect(&dom, id).x, rect(&dom, id).y))
}

/// CSS Box 4 §3: `margin-trim: inline-start` trims the inline-start
/// margin of a float whose margin edge abuts the container's inline-start
/// content edge — the first float, not the one beside it; `block-start`
/// the block-start margin of a float at the container's block-start edge.
#[test]
fn margin_trim_trims_the_floats_at_the_edges() {
    assert_eq!(two_floats(""), [(2, 1), (7, 1)]);
    assert_eq!(two_floats("margin-trim: inline-start"), [(0, 1), (5, 1)]);
    assert_eq!(two_floats("margin-trim: block-start"), [(2, 0), (7, 0)]);
}

/// CSS Overflow 4 §4.4: what follows the clamp point is hidden — the
/// part of a float below the container's first line — and the automatic
/// height ends at the clamp point, the float not extending it.
#[test]
fn line_clamp_hides_a_float_below_the_clamp_point() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let f = boxed(&mut dom, c, "f", "F");
    text(&mut dom, c, "aa bb cc dd");
    let css = ".c { width: 6; line-clamp: 1 } .f { float: left; width: 2; height: 3 } \
               .f { background-color: #ff0000 }";
    let buf = paint(&mut dom, css, 8, 3);
    assert_eq!(rect(&dom, c).height, 1);
    assert_eq!(rect(&dom, f).height, 3);
    let red = |y| buf.cell(0, y).unwrap().bg == rdom_tui::render::Color::Rgb(255, 0, 0);
    assert_eq!((red(0), red(1), red(2)), (true, false, false));
}

/// CSS Overflow 4 §3: `text-overflow` marks the content overflowing the
/// end line box edge — beside a right float, the float's left edge, so
/// the marker is not hidden under the float. (The first word fits beside
/// the float, so the `nowrap` line stays there — CSS 2.1 §9.5 moves a
/// line down only when nothing fits.)
#[test]
fn text_overflow_marks_the_line_box_edge_beside_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    boxed(&mut dom, c, "f", "RRR");
    text(&mut dom, c, "ab cdefghij");
    let css = ".c { width: 10; overflow: hidden; white-space: nowrap; \
               text-overflow: ellipsis } .f { float: right; width: 3; height: 1 }";
    let buf = paint(&mut dom, css, 10, 1);
    assert_eq!(rows(&buf, 10, 1), ["ab cde…RRR"]);
}
