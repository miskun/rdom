//! C8G-FLOAT-MEASURE — one block-flow model for layout and for intrinsic
//! block sizes (CSS Sizing 3 §5.2: a box's content contribution is the
//! size it would have laid out under the constraint): margin collapsing
//! (CSS 2.1 §8.3.1), `box-sizing` and `min-*` / `max-*` (CSS 2.1 §10.7,
//! CSS UI 3 §3.1) and line clamping (CSS Overflow 4 §4) measure as they lay
//! out, with floats or without; a formatting context root beside floats
//! dodges them at the height it gets (§9.5); a float met in inline content
//! settles to its laid-out height; an inline block is shrink-to-fit
//! (§10.3.9).

use super::place::{boxed, text};
use crate::css_phase8::{el, lay_out, paint, rect, rows};
use rdom_tui::prelude::*;

/// `.it` — a flex item of `.row`, its height its content measured before
/// it is laid out (CSS Flexbox §9.4, a block formatting context root) —
/// holding an optional left float `.f` and two paragraphs `.p` ("a",
/// "b"), styled by `css`: its height.
fn item_height(css: &str, float: bool) -> u16 {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let it = el(&mut dom, row, "div", "it");
    if float {
        boxed(&mut dom, it, "f", "F");
    }
    for s in ["a", "b"] {
        let p = el(&mut dom, it, "p", "p");
        text(&mut dom, p, s);
    }
    lay_out(
        &mut dom,
        &format!(
            ".row {{ display: flex; align-items: flex-start }} \
             .f {{ float: left; width: 1 }} {css}"
        ),
        20,
        20,
    );
    rect(&dom, it).height
}

/// CSS 2.1 §8.3.1: the paragraphs' adjoining margins collapse (1, a, 1,
/// b, 1 — five rows, the item a formatting context root keeping its
/// children's outer margins), measured as laid out — with a float or
/// without (both measured 6).
#[test]
fn intrinsic_heights_collapse_margins() {
    assert_eq!(item_height(".p { margin: 1 0 }", false), 5);
    assert_eq!(item_height(".p { margin: 1 0 }", true), 5);
}

/// CSS 2.1 §10.7 / CSS UI 3 §3.1: a paragraph's `min-height` holds it
/// open, and a `border-box` one counts its padding — measured beside a
/// float as without one (the float measurement ignored both: 2 for 4, 6
/// for 8).
#[test]
fn intrinsic_heights_beside_a_float_honour_min_height_and_box_sizing() {
    assert_eq!(item_height(".p { min-height: 2 }", true), 4);
    assert_eq!(
        item_height(
            ".p { padding: 1 0; min-height: 4; box-sizing: border-box }",
            true
        ),
        8
    );
}

/// CSS Overflow 4 §4: a line-clamp container's automatic height ends at
/// its Nth line box, its block descendants' lines counted — measured as
/// laid out (2 for 1, DIVERGENCES' "measured unclamped").
#[test]
fn a_clamped_items_intrinsic_height_ends_at_its_clamp_point() {
    assert_eq!(item_height(".it { line-clamp: 1 }", false), 1);
    assert_eq!(item_height(".it { line-clamp: 1 }", true), 1);
}

/// §9.5: a formatting context root must not overlap a float's margin box
/// at the height it gets. `.o` (`overflow: hidden`) fits at row 0 beside
/// the 2-row `.f1` at its one-row measured height, but laid out 6 wide it
/// wraps to 3 rows and would overlap `.f2` (cleared below `.f1`, 6 wide on
/// row 2): placed again at its height, it goes beside both, at column 6.
#[test]
fn a_formatting_context_root_dodges_floats_at_its_laid_out_height() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    boxed(&mut dom, body, "f1", "A");
    boxed(&mut dom, body, "f2", "B");
    let o = el(&mut dom, body, "div", "o");
    text(&mut dom, o, "aa bb cc dd ee ff");
    lay_out(
        &mut dom,
        ".f1 { float: left; width: 4; height: 2 } \
         .f2 { float: left; clear: left; width: 6; height: 1 } \
         .o { overflow: hidden }",
        10,
        10,
    );
    assert_eq!(rect(&dom, o).x, 6);
}

/// A float met in inline content is placed at its measured height; laid
/// out, an `overflow-x: auto` float's horizontal scrollbar adds a row
/// (CSS Overflow 3 §3), and its exclusion settles to it — the next
/// paragraph's line beside it (it started at column 0 under it).
#[test]
fn a_float_met_in_inline_content_settles_to_its_laid_out_height() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let c = el(&mut dom, body, "div", "c");
    text(&mut dom, c, "x");
    boxed(&mut dom, c, "f", "abcdef");
    let p = el(&mut dom, body, "p", "p");
    text(&mut dom, p, "hello");
    let buf = paint(
        &mut dom,
        ".f { float: left; width: 3; overflow-x: auto }",
        10,
        3,
    );
    let row: String = rows(&buf, 10, 3)[1].chars().skip(3).take(5).collect();
    assert_eq!(row, "hello");
}

/// CSS 2.1 §10.3.9: an inline block's `auto` width is shrink-to-fit —
/// `min(max(min-content, available), max-content)` — so `aaa bbb` in a
/// 5-wide line wraps inside a 5-wide box (it took its max-content 7 and
/// overflowed), and its min-content contribution is its min-content
/// width, 3 (it was 7).
#[test]
fn an_inline_block_is_shrink_to_fit() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let p = el(&mut dom, body, "p", "");
    text(&mut dom, p, "xx ");
    let ib = el(&mut dom, p, "span", "ib");
    text(&mut dom, ib, "aaa bbb");
    let buf = paint(&mut dom, ".ib { display: inline-block }", 5, 3);
    assert_eq!(rows(&buf, 5, 3), ["xx   ", "aaa  ", "bbb  "]);
    assert_eq!(rect(&dom, ib).width, 5);

    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let d = el(&mut dom, body, "div", "d");
    let ib = el(&mut dom, d, "span", "ib");
    text(&mut dom, ib, "aaa bbb");
    lay_out(
        &mut dom,
        ".d { width: min-content } .ib { display: inline-block }",
        20,
        3,
    );
    assert_eq!(rect(&dom, d).width, 3);
}
