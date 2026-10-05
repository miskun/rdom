//! C9-TEXT-INDENT — `text-indent` (CSS Text 3 §8.1): an indent of the
//! first formatted line (and, with `each-line`, of each line after a
//! forced break; `hanging` inverting which), "treated as a margin applied
//! to the start edge of the line box".

use super::{el, lay_out, paint, paint_text, rows, size, text_block};
use rdom_tui::prelude::*;

/// §8.1: the first line's start edge moves in by the indent, which
/// shortens it; later lines are not indented.
#[test]
fn the_first_line_is_indented() {
    assert_eq!(
        paint_text("width: 8; text-indent: 2", "aaa bbb ccc", 8, 2),
        ["  aaa   ", "bbb ccc "]
    );
}

/// §8.1: "the indent is treated as a margin" — a negative one moves the
/// first line's start outside the content box, lengthening it.
#[test]
fn a_negative_indent_outdents() {
    assert_eq!(
        paint_text(
            "width: 8; padding-left: 2; text-indent: -2",
            "aaa bbb ccc ddd",
            10,
            2
        ),
        ["aaa bbb   ", "  ccc ddd "]
    );
}

/// §8.1: "Percentages: refers to block container's own inline size".
#[test]
fn a_percentage_is_of_the_block_width() {
    assert_eq!(
        paint_text("width: 8; text-indent: 50%", "aaa bbb", 8, 2),
        ["    aaa ", "bbb     "]
    );
}

/// §8.1 `hanging`: "inverts which lines are affected" — every line but
/// the first; `each-line`: the lines after a forced break are indented
/// too, not those after a soft wrap.
#[test]
fn hanging_and_each_line_choose_the_lines() {
    assert_eq!(
        paint_text("width: 8; text-indent: 2 hanging", "aaa bbb ccc", 8, 2),
        ["aaa bbb ", "  ccc   "]
    );
    assert_eq!(
        paint_text(
            "width: 6; white-space: pre-line; text-indent: 2 each-line",
            "aa\nbb cc dd",
            6,
            3
        ),
        ["  aa  ", "  bb  ", "cc dd "]
    );
    assert_eq!(
        paint_text(
            "width: 6; white-space: pre-line; text-indent: 2",
            "aa\nbb cc dd",
            6,
            3
        ),
        ["  aa  ", "bb cc ", "dd    "]
    );
}

/// §8.1: the start edge is the inline-start one — the right edge under
/// `direction: rtl` (CSS Writing Modes 4 §2.1).
#[test]
fn an_rtl_line_is_indented_from_the_right() {
    assert_eq!(
        paint_text("width: 8; direction: rtl; text-indent: 2", "aaa", 8, 1),
        ["   aaa  "]
    );
}

/// §8.1: "only lines that are the first formatted line of an element are
/// affected ... the first line of an anonymous block box is only affected
/// if it is the first child of its parent element"; the block child's own
/// first line takes its own (inherited) indent.
#[test]
fn only_the_first_formatted_line_of_each_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("text");
    dom.append_child(b, t).unwrap();
    let p = el(&mut dom, b, "div", "p");
    let pt = dom.create_text_node("block");
    dom.append_child(p, pt).unwrap();
    let m = dom.create_text_node("more");
    dom.append_child(b, m).unwrap();
    let buf = paint(&mut dom, ".b { text-indent: 2 }", 8, 3);
    assert_eq!(rows(&buf, 8, 3), ["  text  ", "  block ", "more    "]);
}

/// The indent is part of the first line's content for intrinsic sizes
/// (CSS Sizing 3 §5.1: the line as laid out), and tab stops count from the
/// content edge, not the indented line start (§4.2).
#[test]
fn intrinsic_sizes_and_tab_stops_see_the_indent() {
    let (mut dom, b, _) = text_block("abc");
    lay_out(&mut dom, ".b { width: max-content; text-indent: 3 }", 10, 1);
    assert_eq!(size(&dom, b).0, 6);
    assert_eq!(
        paint_text("white-space: pre; tab-size: 4; text-indent: 2", "\tx", 8, 1),
        ["    x   "]
    );
}
