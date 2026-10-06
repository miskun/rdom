//! C9-VERTICAL-ALIGN — `vertical-align` (CSS 2.1 §10.8.1, CSS Inline 3
//! §4): where an inline box — an inline element's, generated text's, an
//! inline block's — sits in its line, in whole rows: shifted from its
//! parent's baseline, or aligned with the line box's top or bottom.

use super::{el, lay_out, paint, rows};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;

/// `a<span class=s>b<span class=s>c</span></span>` in a `.b` block,
/// styled `css`, painted 3 × `h`.
fn nested(css: &str, h: u16) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("a");
    dom.append_child(b, t).unwrap();
    let s = el(&mut dom, b, "span", "s");
    let t = dom.create_text_node("b");
    dom.append_child(s, t).unwrap();
    let inner = el(&mut dom, s, "span", "s");
    let t = dom.create_text_node("c");
    dom.append_child(inner, t).unwrap();
    let buf = paint(&mut dom, css, 3, h);
    rows(&buf, 3, h)
}

/// §10.8.1: `super` raises the box to the parent's superscript position,
/// `sub` lowers it to the subscript one — one row each on a grid, the
/// line growing to hold them — measured from the parent's baseline, so
/// nested shifts add up; a length raises by that many rows (negative
/// lowers).
#[test]
fn sub_super_and_lengths_shift_from_the_parents_baseline() {
    assert_eq!(
        nested(".s { vertical-align: super }", 3),
        ["  c", " b ", "a  "]
    );
    assert_eq!(
        nested(".s { vertical-align: sub }", 3),
        ["a  ", " b ", "  c"]
    );
    assert_eq!(
        nested(".s { vertical-align: 2 } .s .s { vertical-align: -1 }", 3),
        [" b ", "  c", "a  "]
    );
}

/// §10.8.1: "<percentage> Raise (positive value) or lower (negative
/// value) the box by this distance (a percentage of the line-height
/// value)".
#[test]
fn a_percentage_is_of_the_elements_line_height() {
    assert_eq!(
        nested(
            ".s { line-height: 2; vertical-align: 50% } .s .s { vertical-align: 0 }",
            3
        ),
        [" bc", "a  ", "   "]
    );
}

/// The row of `B` in `a<span class=m>B</span>`, the block `line-height:
/// 5` (its baseline on row 2), the inline block `.m` one row of line
/// height styled `m`.
fn atom_row(m: &str) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("a");
    dom.append_child(b, t).unwrap();
    let span = el(&mut dom, b, "span", "m");
    let t = dom.create_text_node("B");
    dom.append_child(span, t).unwrap();
    let css = format!(
        ".b {{ width: 2; line-height: 5 }} \
         .m {{ display: inline-block; line-height: 1; {m} }}"
    );
    let buf = paint(&mut dom, &css, 2, 5);
    let rows = rows(&buf, 2, 5);
    assert_eq!(
        rows[2],
        format!("a{}", &rows[2][1..]),
        "`a` on the baseline row"
    );
    rows.iter()
        .position(|r| r.contains('B'))
        .unwrap_or_else(|| panic!("{m}: {rows:?}"))
}

/// §10.8.1 on an inline block (its baseline its content's row): `top` /
/// `bottom` align its margin box with the line box's top / bottom,
/// `middle` its middle row with the parent's baseline, `text-top` /
/// `text-bottom` its top / bottom with the parent's glyph row.
#[test]
fn an_inline_block_aligns_by_each_keyword() {
    let content_top = "height: 3";
    let content_bottom = "padding-top: 2";
    let cases = [
        (content_top, "baseline", 2),
        (content_top, "top", 0),
        (content_top, "middle", 1),
        (content_top, "text-bottom", 0),
        (content_bottom, "bottom", 4),
        (content_bottom, "middle", 3),
        (content_bottom, "text-top", 4),
        (content_bottom, "baseline", 2),
    ];
    for (geometry, va, row) in cases {
        let m = format!("{geometry}; vertical-align: {va}");
        assert_eq!(atom_row(&m), row, "{m}");
    }
}

/// §10.8.1 on an inline element: the box aligned is its line-height box
/// (`line-height: 3`, a row of leading above and below its glyph row).
#[test]
fn an_inline_element_aligns_its_line_height_box() {
    for (va, row) in [
        ("baseline", 2),
        ("text-top", 3),
        ("text-bottom", 1),
        ("middle", 2),
        ("top", 1),
        ("bottom", 3),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "b");
        let t = dom.create_text_node("a");
        dom.append_child(b, t).unwrap();
        let s = el(&mut dom, b, "span", "s");
        let t = dom.create_text_node("b");
        dom.append_child(s, t).unwrap();
        let css = format!(
            ".b {{ width: 2; line-height: 5 }} .s {{ line-height: 3; vertical-align: {va} }}"
        );
        let buf = paint(&mut dom, &css, 2, 5);
        let rows = rows(&buf, 2, 5);
        assert_eq!(rows[2].chars().next(), Some('a'), "{va}: {rows:?}");
        let b_row = rows.iter().position(|r| r.ends_with('b'));
        assert_eq!(b_row, Some(row), "{va}: {rows:?}");
    }
}

/// §10.8: "the line box height is the distance between the uppermost box
/// top and the lowermost box bottom" — a `top` / `bottom` box taller than
/// the rest grows the line away from the edge it is aligned with.
#[test]
fn a_tall_top_or_bottom_box_grows_the_line() {
    for (va, expected) in [("top", ["aB", "  ", "  "]), ("bottom", [" B", "  ", "a "])] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "b");
        let t = dom.create_text_node("a");
        dom.append_child(b, t).unwrap();
        let m = el(&mut dom, b, "span", "m");
        let t = dom.create_text_node("B");
        dom.append_child(m, t).unwrap();
        let css = format!(
            ".b {{ width: 2 }} .m {{ display: inline-block; height: 3; vertical-align: {va} }}"
        );
        let buf = paint(&mut dom, &css, 2, 3);
        assert_eq!(rows(&buf, 2, 3), expected, "{va}");
    }
}

/// CSS 2.1 §12.1: a `::before` is an inline box of its host, aligned by
/// its own `vertical-align`; the caret sits on a shifted text's row.
#[test]
fn generated_text_and_the_caret_follow_the_shift() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("a");
    dom.append_child(b, t).unwrap();
    let s = el(&mut dom, b, "span", "s");
    let st = dom.create_text_node("bc");
    dom.append_child(s, st).unwrap();
    let css = ".b::before { content: '^'; vertical-align: super } .s { vertical-align: sub }";
    let buf = paint(&mut dom, css, 4, 3);
    assert_eq!(rows(&buf, 4, 3), ["^   ", " a  ", "  bc"]);
    lay_out(&mut dom, css, 4, 3);
    assert_eq!(cell_of_position(&dom, Position::new(st, 1)), Some((3, 2)));
    assert_eq!(dom.position_at(3, 2), Some(Position::new(st, 1)));
}

/// HTML §15.3.4: the UA sheet styles `sub { vertical-align: sub }` and
/// `sup { vertical-align: super }`, `line-height: normal` on both.
#[test]
fn sub_and_sup_shift_by_default() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("x");
    dom.append_child(b, t).unwrap();
    let sup = el(&mut dom, b, "sup", "");
    let t = dom.create_text_node("2");
    dom.append_child(sup, t).unwrap();
    let sub = el(&mut dom, b, "sub", "");
    let t = dom.create_text_node("i");
    dom.append_child(sub, t).unwrap();
    let buf = paint(&mut dom, "sup, sub { color: red }", 3, 3);
    assert_eq!(rows(&buf, 3, 3), [" 2 ", "x  ", "  i"]);
}

/// Append the text `s` to `parent`.
fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

/// C9G-ONE-BASELINE. CSS 2.1 §10.8.1: an inline-block's baseline is "the
/// baseline of its last line box in the normal flow" — the row its text
/// sits on, not its content's last row: `H<sub>2</sub>O` is one line two
/// rows tall (the `2` lowered a row, HTML's `sub { vertical-align: sub }`),
/// its baseline the `H O` row, which sits on the `a b` row.
#[test]
fn an_inline_blocks_baseline_is_its_last_lines_text_row() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    text(&mut dom, b, "a ");
    let ib = el(&mut dom, b, "span", "ib");
    text(&mut dom, ib, "H");
    let sub = el(&mut dom, ib, "sub", "");
    text(&mut dom, sub, "2");
    text(&mut dom, ib, "O");
    text(&mut dom, b, " b");
    let buf = paint(&mut dom, ".ib { display: inline-block }", 8, 3);
    assert_eq!(rows(&buf, 8, 3), ["a H O b ", "   2    ", "        "]);
}

/// C9G-ONE-BASELINE. CSS Box Alignment 3 §9.1: a block container's first
/// baseline is its first in-flow line box's — here the anonymous line `t`
/// before a block child whose `line-height: 3` adds leading — so
/// `align-items: baseline` puts the sibling's `z` on `t`'s row.
#[test]
fn a_leading_anonymous_line_holds_the_first_baseline() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "");
    text(&mut dom, a, "t");
    let u = el(&mut dom, a, "div", "u");
    text(&mut dom, u, "u");
    let z = el(&mut dom, f, "div", "");
    text(&mut dom, z, "z");
    let buf = paint(
        &mut dom,
        ".f { display: flex; align-items: baseline; width: 4 } .u { line-height: 3 }",
        4,
        4,
    );
    assert_eq!(rows(&buf, 4, 4), ["tz  ", "    ", "u   ", "    "]);
}
