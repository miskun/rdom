//! C9-LINE-HEIGHT — `line-height` (CSS Inline 3 §5.1, CSS 2.1 §10.8): an
//! inline box is as tall as its line height, its glyph row placed by
//! half-leading (§10.8.1), and a line box as tall as the inline boxes on
//! it, the block's strut among them. On a terminal grid the line height
//! is whole rows: `normal` and `1` are one, a number scales the font size
//! (one row), a length is rows.

use super::{el, lay_out, paint, paint_text, rect, rows, text_block};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;

/// CSS 2.1 §10.8.1: "the leading L = line-height − AD ... A/2 above, A/2
/// below" — the two leading rows of `line-height: 3` sit one above and
/// one below each glyph row, and each line box is three rows tall.
#[test]
fn each_line_is_line_height_rows_with_half_leading() {
    assert_eq!(
        paint_text("width: 2; line-height: 3", "aa bb", 2, 6),
        ["  ", "aa", "  ", "  ", "bb", "  "]
    );
}

/// §10.8.1 with an odd leading: rdom places the glyph row
/// `floor((L − 1) / 2)` rows down, the extra leading row below it
/// (DIVERGENCES §2).
#[test]
fn an_odd_leading_puts_the_extra_row_below() {
    assert_eq!(
        paint_text("width: 2; line-height: 2", "aa bb", 2, 4),
        ["aa", "  ", "bb", "  "]
    );
    assert_eq!(
        paint_text("width: 2; line-height: 4", "aa", 2, 4),
        ["  ", "aa", "  ", "  "]
    );
}

/// CSS Inline 3 §5.1: `normal` and the number 1 are the font's height —
/// one row; a number "multiplied by the element's font size" (one row)
/// and a percentage "relative to the font size" round onto the grid;
/// a length is rows; a height below one row still takes the glyph's row.
#[test]
fn the_values_map_onto_whole_rows() {
    for (value, height) in [
        ("normal", 2),
        ("1", 2),
        ("1.5", 4),
        ("150%", 4),
        ("3", 6),
        ("calc(1 + 1)", 4),
        ("0", 2),
        ("0.4", 2),
    ] {
        let (mut dom, b, _) = text_block("aa bb");
        lay_out(
            &mut dom,
            &format!(".b {{ width: 2; line-height: {value} }}"),
            2,
            8,
        );
        assert_eq!(rect(&dom, b).height, height, "line-height: {value}");
    }
}

/// CSS 2.1 §10.8.1: "each line box starts with a zero-width inline box
/// with the element's font and line height properties" (the strut), so
/// an empty line between two forced breaks is a line height tall, and an
/// inline box's own `line-height` grows only the lines it is on — its
/// content's, even nested in an inline box of one row.
#[test]
fn the_strut_and_an_inline_boxs_own_line_height() {
    assert_eq!(
        paint_text("white-space: pre-line; line-height: 2", "a\n\nb", 1, 6),
        ["a", " ", " ", " ", "b", " "]
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("aa ");
    dom.append_child(b, t).unwrap();
    let s = el(&mut dom, b, "span", "s");
    let inner = el(&mut dom, s, "b", "");
    let st = dom.create_text_node("bb");
    dom.append_child(inner, st).unwrap();
    let t = dom.create_text_node(" cc");
    dom.append_child(b, t).unwrap();
    let css = ".b { width: 2 } .s { line-height: 3 } b { line-height: 1 }";
    let buf = paint(&mut dom, css, 2, 5);
    assert_eq!(rows(&buf, 2, 5), ["aa", "  ", "bb", "  ", "cc"]);
}

/// CSS 2.1 §12.1: a `::before` is an inline box of its host, with its
/// own line height; CSS Display 3 §2.5: the text of a `display: contents`
/// element beside a block box inherits its line height (its anonymous
/// inline box does).
#[test]
fn generated_content_and_a_split_inline_box_keep_their_line_height() {
    let (mut dom, _, _) = text_block("aa bb");
    let css = ".b { width: 4 } .b::before { content: '> '; line-height: 3 }";
    let buf = paint(&mut dom, css, 4, 4);
    assert_eq!(rows(&buf, 4, 4), ["    ", "> aa", "    ", "bb  "]);

    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let s = el(&mut dom, b, "span", "s");
    let t = dom.create_text_node("aa");
    dom.append_child(s, t).unwrap();
    let d = el(&mut dom, s, "div", "d");
    let t = dom.create_text_node("x");
    dom.append_child(d, t).unwrap();
    let t = dom.create_text_node("bb");
    dom.append_child(s, t).unwrap();
    let css = ".b { width: 2 } .s { display: contents; line-height: 3 } .d { line-height: 1 }";
    let buf = paint(&mut dom, css, 2, 7);
    assert_eq!(rows(&buf, 2, 7), ["  ", "aa", "  ", "x ", "  ", "bb", "  "]);
}

/// CSS 2.1 §10.8.1: an inline-block's baseline is "the baseline of its
/// last line box in the normal flow" — the glyph row of its last line,
/// not the last row of its content, so the leading below it hangs under
/// the outer line's baseline.
#[test]
fn an_inline_blocks_baseline_is_its_last_lines_glyph_row() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("x");
    dom.append_child(b, t).unwrap();
    let ib = el(&mut dom, b, "span", "ib");
    let it = dom.create_text_node("y");
    dom.append_child(ib, it).unwrap();
    let t = dom.create_text_node("z");
    dom.append_child(b, t).unwrap();
    let css = ".ib { display: inline-block; line-height: 3 }";
    let buf = paint(&mut dom, css, 3, 3);
    assert_eq!(rows(&buf, 3, 3), ["   ", "xyz", "   "]);
    assert_eq!(rect(&dom, b).height, 3);
}

/// The caret and hit-testing follow the line boxes (CSS 2.1 §10.8): the
/// second line's glyph row is row 4 at `line-height: 3`, and a click on
/// a leading row lands in the line it belongs to.
#[test]
fn the_caret_and_hit_testing_use_the_glyph_rows() {
    let (mut dom, _, t) = text_block("aa bb");
    lay_out(&mut dom, ".b { width: 2; line-height: 3 }", 2, 6);
    assert_eq!(cell_of_position(&dom, Position::new(t, 3)), Some((0, 4)));
    for row in 3..6 {
        assert_eq!(
            dom.position_at(0, row),
            Some(Position::new(t, 3)),
            "row {row}"
        );
    }
    assert_eq!(dom.position_at(0, 0), Some(Position::new(t, 0)));
}

/// CSS Overflow 4 §4: a clamped box ends after its Nth line box — a line
/// height tall each; a scroll container's scrollable overflow holds its
/// line boxes whole (CSS Overflow 3 §2.2).
#[test]
fn line_clamp_and_scroll_extents_count_line_boxes() {
    let (mut dom, b, _) = text_block("aa bb cc");
    lay_out(
        &mut dom,
        ".b { width: 2; line-height: 2; line-clamp: 2 }",
        2,
        8,
    );
    assert_eq!(rect(&dom, b).height, 4);
    let (mut dom, b, _) = text_block("aa bb cc");
    lay_out(
        &mut dom,
        ".b { width: 2; height: 2; overflow: hidden; line-height: 2 }",
        2,
        8,
    );
    assert_eq!(dom.node(b).scroll_height(), Some(6));
}

/// CSS Box Alignment 3 §9.1: a flex item's first baseline is its first
/// line's — the glyph row, below the line's leading — for the anonymous
/// item of a flex container's text too.
#[test]
fn a_flex_items_baseline_is_its_first_lines_glyph_row() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let t = dom.create_text_node("aa");
    dom.append_child(f, t).unwrap();
    let i = el(&mut dom, f, "span", "i");
    let t = dom.create_text_node("b");
    dom.append_child(i, t).unwrap();
    let css = ".f { display: flex; align-items: baseline; line-height: 3 } .i { line-height: 1 }";
    let buf = paint(&mut dom, css, 3, 3);
    assert_eq!(rows(&buf, 3, 3), ["   ", "aab", "   "]);
}

/// CSS Values 4 §6.1.1: `lh` is "equal to the computed value of the
/// line-height property of the element on which it is used" — in
/// `line-height` itself, of its parent — and `rlh` the root element's.
#[test]
fn lh_and_rlh_follow_the_line_height() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let r = el(&mut dom, root, "div", "r");
    let a = el(&mut dom, r, "div", "a");
    let b = el(&mut dom, r, "div", "b");
    let c = el(&mut dom, b, "div", "c");
    let css = ".r { line-height: 2 } \
               .a { line-height: 3; height: 2lh } \
               .b { line-height: 2lh; height: 1lh } \
               .c { line-height: 5; height: 1rlh }";
    lay_out(&mut dom, css, 10, 20);
    assert_eq!(rect(&dom, a).height, 6);
    assert_eq!(rect(&dom, b).height, 4);
    assert_eq!(rect(&dom, c).height, 2);
}
