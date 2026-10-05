//! C8G-PAINT-PHASES — in-flow content paints in CSS 2.1 Appendix E's
//! phases within each paint unit (a stacking context, a `z-index: auto`
//! positioned box, a float, an atomic box): the in-flow block-level
//! boxes' backgrounds and borders (step 4), then the floats (step 5),
//! then the inline content (step 7) — not each in-flow box whole.

use super::{el, paint};
use rdom_tui::prelude::*;
use rdom_tui::style::Color;

const RED: Color = Color::Rgb(200, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 200);

/// `parent > tag.class` holding `text`.
fn boxed(dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str, text: &str) -> NodeId {
    let id = el(dom, parent, tag, class);
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    id
}

/// Appendix E step 7 after step 5: a paragraph's text overflowing into a
/// float beside it paints over the float — the glyphs on the float's
/// background — where the float used to paint over it.
#[test]
fn inline_content_paints_over_a_float() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    boxed(&mut dom, body, "div", "f", "F");
    boxed(&mut dom, body, "p", "p", "abcdefghij");
    let buf = paint(
        &mut dom,
        ".f { float: right; width: 3; height: 2; background-color: rgb(0, 0, 200) } \
         .p { width: 4 }",
        10,
        2,
    );
    let row: String = (0..10)
        .map(|x| buf.cell(x, 0).unwrap().symbol().to_string())
        .collect();
    assert_eq!(row, "abcdefghij");
    for x in 7..10 {
        assert_eq!(buf.cell(x, 0).unwrap().bg, BLUE, "column {x}");
    }
    assert_eq!(buf.cell(7, 1).unwrap().symbol(), " ", "the float's box");
}

/// Appendix E step 7 after step 4: a block's second line overflowing its
/// one-row height shows on the next block's background, which used to
/// cover it (DIVERGENCES §2, C4G-SHADOW-ORDER's gap).
#[test]
fn a_later_blocks_background_stays_under_earlier_overflowing_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    boxed(&mut dom, body, "div", "a", "line1 line2");
    boxed(&mut dom, body, "div", "b", "");
    let buf = paint(
        &mut dom,
        ".a { width: 5; height: 1 } .b { height: 1; background-color: rgb(200, 0, 0) }",
        10,
        2,
    );
    let row: String = (0..5)
        .map(|x| buf.cell(x, 1).unwrap().symbol().to_string())
        .collect();
    assert_eq!(row, "line2");
    assert_eq!(buf.cell(0, 1).unwrap().bg, RED, "the block's background");
}

/// A float inside a `z-index: auto` positioned box is that box's (it
/// paints as if it created a stacking context): its background first,
/// then the float, then its text — the float was painted before the box
/// and covered by its background.
#[test]
fn a_float_in_a_positioned_box_paints_above_its_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let r = el(&mut dom, body, "div", "r");
    boxed(&mut dom, r, "div", "f", "F");
    let t = dom.create_text_node("text");
    dom.append_child(r, t).unwrap();
    let buf = paint(
        &mut dom,
        ".r { position: relative; background-color: rgb(200, 0, 0) } \
         .f { float: left; width: 2 }",
        10,
        1,
    );
    let row: String = (0..6)
        .map(|x| buf.cell(x, 0).unwrap().symbol().to_string())
        .collect();
    assert_eq!(row, "F text");
}

/// A float inside an inline block is the atom's: painted over the atom's
/// background, at its turn in the line.
#[test]
fn a_float_in_an_inline_block_paints_above_its_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let p = el(&mut dom, body, "p", "");
    let a = el(&mut dom, p, "span", "a");
    boxed(&mut dom, a, "div", "f", "F");
    let t = dom.create_text_node("x");
    dom.append_child(a, t).unwrap();
    let buf = paint(
        &mut dom,
        ".a { display: inline-block; background-color: rgb(200, 0, 0) } \
         .f { float: left; width: 1 }",
        10,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "F");
    assert_eq!(buf.cell(1, 0).unwrap().symbol(), "x");
    assert_eq!(buf.cell(1, 0).unwrap().bg, RED);
}

/// The phases keep atomic boxes atomic: an inline block's background and
/// text paint together at its turn in the line, over the line's earlier
/// text that overflows into it — not under a later line's text.
#[test]
fn an_atom_paints_whole_at_its_turn() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let p = el(&mut dom, body, "p", "");
    let t = dom.create_text_node("ab ");
    dom.append_child(p, t).unwrap();
    boxed(&mut dom, p, "span", "a", "XY");
    let buf = paint(
        &mut dom,
        ".a { display: inline-block; background-color: rgb(200, 0, 0) }",
        10,
        1,
    );
    let row: String = (0..5)
        .map(|x| buf.cell(x, 0).unwrap().symbol().to_string())
        .collect();
    assert_eq!(row, "ab XY");
    assert_eq!(buf.cell(3, 0).unwrap().bg, RED);
    assert_ne!(buf.cell(2, 0).unwrap().bg, RED);
}
