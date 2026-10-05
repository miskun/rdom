//! Floats in intrinsic sizes (CSS Sizing 3 §5.1 / §5.2; CSS 2.1 §10.6.7
//! for the content height of a block formatting context root sized from
//! its content).

use super::place::{boxed, text};
use crate::css_phase8::{el, lay_out, rect};
use rdom_tui::prelude::*;

/// `.c` — a flex item of a row flex container `.o`, whose height is its
/// content height measured before it is laid out (CSS Flexbox §9.4: its
/// hypothetical cross size; a block formatting context root, §4) —
/// holding a left float `.f` and the text `t`: its height.
fn item_height(f: &str, t: &str) -> u16 {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let o = el(&mut dom, root, "div", "o");
    let c = el(&mut dom, o, "div", "c");
    boxed(&mut dom, c, "f", "F");
    text(&mut dom, c, t);
    lay_out(
        &mut dom,
        &format!(
            ".o {{ display: flex; align-items: flex-start }} .c {{ width: 10 }} \
             .f {{ float: left; {f} }}"
        ),
        10,
        8,
    );
    rect(&dom, c).height
}

/// §10.6.7: the content height reaches the bottom of the float — 3 rows
/// for a 3-row float beside one line.
#[test]
fn a_float_taller_than_the_lines_sets_the_content_height() {
    assert_eq!(item_height("width: 2; height: 3", "word"), 3);
}

/// The lines beside a float are shorter (§9.5), so the text takes more
/// of them: `aaa bbb ccc` beside a 6-wide, 2-row float packs `aaa`,
/// `bbb` into the 4 columns left, then `ccc` below — 3 rows, where the
/// full width would take 2.
#[test]
fn shortened_lines_add_rows_to_the_content_height() {
    assert_eq!(item_height("width: 6; height: 2", "aaa bbb ccc"), 3);
}

/// A block container whose float is a block-level sibling of its
/// paragraph: the paragraph's lines are shortened in the measurement too.
#[test]
fn a_float_beside_a_block_child_counts() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let o = el(&mut dom, root, "div", "o");
    let c = el(&mut dom, o, "div", "c");
    boxed(&mut dom, c, "f", "F");
    boxed(&mut dom, c, "p", "aaa bbb ccc");
    lay_out(
        &mut dom,
        ".o { display: flex; align-items: flex-start } .c { width: 10 } \
         .f { float: left; width: 6; height: 2 }",
        10,
        8,
    );
    assert_eq!(rect(&dom, c).height, 3);
}

/// CSS Sizing 3 §5.1: the max-content width is the content laid out with
/// no soft wrap — the float beside the text on one line (3 + 4); the
/// min-content width the widest piece (§4.2), the float or the word.
#[test]
fn a_float_beside_text_adds_to_the_max_content_width() {
    let widths = |extra: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "div", "c");
        let i = el(&mut dom, c, "div", "i");
        boxed(&mut dom, i, "f", "");
        boxed(&mut dom, i, "p", "abcd");
        lay_out(
            &mut dom,
            &format!(
                ".c {{ display: flex; width: 20 }} .f {{ float: left; width: 3; height: 1 }} \
                 .i {{ {extra} }}"
            ),
            20,
            4,
        );
        rect(&dom, i).width
    };
    assert_eq!(widths(""), 7);
    assert_eq!(widths("width: min-content"), 4);
}
