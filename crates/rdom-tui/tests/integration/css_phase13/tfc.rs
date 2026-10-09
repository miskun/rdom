//! The table formatting context (C13-TFC; CSS 2.1 §17, CSS Tables 3):
//! tables built from `display` values on any element.

use rdom_tui::{HitTestExt, LayoutRect, TuiNodeExt};

use super::{by_id, doc, paint, rows};

/// The `display` values every test here builds its tables from.
const PARTS: &str = ".t { display: table } .it { display: inline-table } \
    .r { display: table-row } .c { display: table-cell } \
    .g { display: table-row-group } .h { display: table-header-group } \
    .f { display: table-footer-group } .cap { display: table-caption } \
    .col { display: table-column }";

fn css(extra: &str) -> String {
    format!("{PARTS} {extra}")
}

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

// ── The grid ───────────────────────────────────────────────────────

/// CSS 2.1 §17.5.2.2: a column is as wide as its widest cell, in every
/// row; an `auto`-width table is as wide as its columns (shrink-to-fit),
/// not its containing block.
#[test]
fn columns_line_up_and_an_auto_table_shrinks_to_fit() {
    let mut dom = doc(
        r#"<div><div id="t" class="t"><div class="r"><div class="c">Alice</div><div class="c">30</div></div><div class="r"><div class="c">Bo</div><div class="c">251</div></div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(".c { padding: 0 1 }"), 30, 3);
    assert_eq!(rows(&buf)[..2], [" Alice  30", " Bo     251"]);
    assert_eq!(rect(&dom, "t").width, 12);
}

/// §17.5.2.2 / CSS Tables 3 "distributing width": in less room than
/// its max-content width a table takes the room, its columns between
/// their min- and max-content widths — the slack shared by how much each
/// can grow — and a cell's text wraps in its column.
#[test]
fn a_narrow_table_squeezes_its_columns_toward_min_content() {
    let mut dom = doc(
        r#"<div class="w"><div id="t" class="t"><div class="r"><div class="c">aa bb</div><div class="c">c</div></div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(".w { width: 4 }"), 10, 3);
    assert_eq!(rows(&buf)[..2], ["aa c", "bb"]);
    assert_eq!(rect(&dom, "t").width, 4);
}

/// HTML §4.9.11 (element names ASCII case-insensitive, HTML §4.9): a
/// `TD` made by `create_element("TD")` spans its `colspan` columns in
/// layout, as the column selectors place it (one span reader for both).
#[test]
fn an_upper_case_td_spans_its_colspan() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r" id="r0"></div><div class="r"><div class="c" id="a">a</div><div class="c" id="b">b</div></div></div></div>"#,
    );
    let r0 = by_id(&dom, "r0");
    let td = dom.create_element("TD");
    dom.set_attribute(td, "class", "c").unwrap();
    dom.set_attribute(td, "colspan", "2").unwrap();
    let text = dom.create_text_node("wide-cell");
    dom.append_child(td, text).unwrap();
    dom.append_child(r0, td).unwrap();
    paint(&mut dom, &css(""), 30, 3);
    let (a, b) = (rect(&dom, "a"), rect(&dom, "b"));
    let wide = dom.node(td).layout_rect().unwrap();
    assert_eq!((wide.x, wide.width), (a.x, (b.x - a.x) as u16 + b.width));
}

/// HTML §4.9.12.1 with CSS Tables 3 §3.3: a `colspan` cell spans its
/// columns, its excess width spread over them; a `rowspan` cell spans its
/// rows, the next row's cells placed beside it.
#[test]
fn colspan_and_rowspan_place_cells_in_the_grid() {
    let mut dom = doc(
        r#"<div><table><tr><td colspan="2">wide</td><td id="r" rowspan="2">R</td></tr><tr><td>a</td><td id="b">b</td></tr></table></div>"#,
    );
    let buf = paint(
        &mut dom,
        "table { display: table } tr { display: table-row } td { display: table-cell; padding: 0 }",
        20,
        3,
    );
    assert_eq!(rows(&buf)[..2], ["wideR", "a b"]);
    assert_eq!((rect(&dom, "r").x, rect(&dom, "r").height), (4, 2));
    assert_eq!(rect(&dom, "b").x, 2);
}

// ── Anonymous table boxes ──────────────────────────────────────────

/// CSS 2.1 §17.2.1: stray text in a table is wrapped in an anonymous row
/// and cell, cells directly in a table in an anonymous row, text in a row
/// in an anonymous cell — and white space between proper table children
/// generates no box. The anonymous cells are cells of the grid: `loose`
/// widens the first column, which `a` sits in.
#[test]
fn anonymous_rows_and_cells_wrap_stray_content() {
    let mut dom = doc(
        "<div><div class=\"t\">\n  <div class=\"r\">a<div class=\"c\">b</div>c</div>\n  loose<div class=\"c\">x</div>\n</div></div>",
    );
    let buf = paint(&mut dom, &css(""), 20, 3);
    assert_eq!(rows(&buf)[..2], ["a    bc", "loosex"]);
}

/// CSS 2.1 §17.2: the first `table-header-group`'s rows come first, the
/// first `table-footer-group`'s last, wherever they are in the tree.
#[test]
fn header_rows_come_first_and_footer_rows_last() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="f"><div class="r"><div class="c">F</div></div></div><div class="g"><div class="r"><div class="c">B</div></div></div><div class="h"><div class="r"><div class="c">H</div></div></div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(""), 10, 4);
    assert_eq!(rows(&buf)[..3], ["H", "B", "F"]);
}

// ── Captions ───────────────────────────────────────────────────────

/// CSS 2.1 §17.4 / §17.4.1: captions sit outside the table box — above
/// it, or below it with `caption-side: bottom` — as wide as the table,
/// which its border and background stay around.
#[test]
fn captions_sit_above_and_below_the_table_box() {
    let mut dom = doc(
        r#"<div><div id="t" class="t"><div class="cap">Top</div><div class="cap bot">Bot</div><div class="r"><div class="c">cell</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(".t { border: solid } .bot { caption-side: bottom }"),
        12,
        6,
    );
    assert_eq!(
        rows(&buf)[..5],
        ["Top", "┌────┐", "│cell│", "└────┘", "Bot"]
    );
    assert_eq!(rect(&dom, "t"), LayoutRect::new(0, 0, 6, 5));
}

/// CSS 2.1 §17.4: `overflow` applies to the table box, not the wrapper
/// around it and its captions — the caption is not clipped, the border
/// is the table box's, and a scroll container's scrollbar runs down
/// beside the table box's rows only. The resizer sits in the table box's
/// corner; the caption is hit where it shows.
#[test]
fn a_clipping_table_clips_its_table_box_not_its_captions() {
    let markup = r#"<div><div id="t" class="t"><div id="cap" class="cap">Caption</div><div class="r"><div class="c">a</div></div><div class="r"><div class="c">b</div></div></div></div>"#;
    let mut dom = doc(markup);
    let buf = paint(
        &mut dom,
        &css(".t { overflow: hidden; border: solid; width: 9 }"),
        12,
        6,
    );
    assert_eq!(
        rows(&buf)[..5],
        [
            "Caption",
            "┌─────────┐",
            "│a        │",
            "│b        │",
            "└─────────┘"
        ]
    );
    // The caption is hit where it shows.
    assert_eq!(dom.hit_test(2, 0), Some(by_id(&dom, "cap")));
    let mut dom = doc(markup);
    let buf = paint(
        &mut dom,
        &css(".t { overflow-y: scroll; border: solid; width: 9; resize: both }"),
        12,
        6,
    );
    let painted = rows(&buf);
    assert_eq!(painted[0], "Caption", "{painted:#?}");
    assert_eq!(painted[1], "┌─────────┐", "{painted:#?}");
    // The bar is in the table box's right column, inside its border.
    let bar = |row: &str| row.chars().nth(9).unwrap_or(' ');
    assert!(painted[2..4].iter().all(|r| bar(r) != ' '), "{painted:#?}");
    assert_eq!(painted[4], "└─────────┘", "{painted:#?}");
}

// ── Width algorithms ───────────────────────────────────────────────

/// CSS 2.1 §17.5.2.1: `table-layout: fixed` sizes the columns from the
/// first row's cell widths and the table's width — the later rows' content
/// takes no part (the long word overflows its cell).
#[test]
fn fixed_layout_reads_the_first_row_only() {
    let mut dom = doc(
        r#"<div><div class="t fx"><div class="r"><div id="a" class="c w4">a</div><div id="b" class="c">b</div></div><div class="r"><div id="l" class="c">abcdefghij</div><div class="c">c</div></div></div></div>"#,
    );
    paint(
        &mut dom,
        &css(".fx { table-layout: fixed; width: 12 } .w4 { width: 4 }"),
        20,
        3,
    );
    assert_eq!(rect(&dom, "a").width, 4);
    assert_eq!((rect(&dom, "b").x, rect(&dom, "b").width), (4, 8));
    assert_eq!(rect(&dom, "l").width, 4);
}

/// CSS Tables 3 "distributing width": a percent column takes its share of
/// the table's width, a length-constrained one its width, the rest goes
/// to the auto columns.
#[test]
fn percent_and_length_columns_take_their_widths() {
    let mut dom = doc(
        r#"<div><div class="t w20"><div class="r"><div id="p" class="c p50">p</div><div id="l" class="c w5">l</div><div id="a" class="c">a</div></div></div></div>"#,
    );
    paint(
        &mut dom,
        &css(".w20 { width: 20 } .p50 { width: 50% } .w5 { width: 5 }"),
        30,
        2,
    );
    assert_eq!(rect(&dom, "p").width, 10);
    assert_eq!(rect(&dom, "l").width, 5);
    assert_eq!(rect(&dom, "a").width, 5);
}

// ── Borders ────────────────────────────────────────────────────────

/// CSS 2.1 §17.6.1: in the separated model each cell keeps its border,
/// and `border-spacing` sits between the cells and between them and the
/// table's edges.
#[test]
fn separated_borders_are_spaced_by_border_spacing() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="c">a</div><div class="c">b</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(".t { border-spacing: 1 0 } .c { border: solid }"),
        12,
        4,
    );
    assert_eq!(rows(&buf)[..3], [" ┌─┐ ┌─┐", " │a│ │b│", " └─┘ └─┘"]);
}

/// CSS 2.1 §17.6.2: in the collapsing model neighbours share one border
/// line, the table's own on the outer lines, and the junctions join
/// (CSS Tables 3 §11.5's winners, paint's junction pass).
#[test]
fn collapsed_borders_share_the_grid_lines() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="c">a</div><div class="c">b</div></div><div class="r"><div class="c">c</div><div class="c">d</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(".t { border-collapse: collapse; border: solid } .c { border: solid; padding: 0 1 }"),
        12,
        6,
    );
    assert_eq!(
        rows(&buf)[..5],
        [
            "┌───┬───┐",
            "│ a │ b │",
            "├───┼───┤",
            "│ c │ d │",
            "└───┴───┘"
        ]
    );
}

/// ACID-FIX-7 (found by acid tile 14). CSS 2.1 §17.6.2 / CSS Tables 3
/// §11.5: the `double` left border of a cell wins the line between it and
/// its neighbour on the cell's row, and that line runs to the junctions at
/// its ends — DIVERGENCES §1: "the glyph joins every direction's line". A
/// cell whose top is `none` does not cover the table's top line, so its
/// side's contributions stopped short of the junction: the junction drew
/// the losing `solid` side's arm (`┬`), and the corner took the table's
/// colour (`┐` in red) instead of the `double` side's.
#[test]
fn a_junction_joins_the_line_that_won_below_it() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="c">a</div><div class="c d">b</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(
            ".t { border-collapse: collapse; border: solid rgb(255, 0, 0) } \
              .c { border: solid; padding: 0 1 } \
              .d { border-style: double; border-top-style: none }",
        ),
        12,
        4,
    );
    assert_eq!(rows(&buf)[..3], ["┌───╥───╖", "│ a ║ b ║", "└───╩═══╝"]);
    // The corner's winner is the cell's `double` side, in the cell's
    // (default) colour, not the table's red.
    let corner = buf.cell(8, 0).unwrap();
    assert_eq!(corner.fg, rdom_tui::render::Color::Reset);
}

// ── Heights and collapse ───────────────────────────────────────────

/// CSS 2.1 §17.5.3: a table taller than its rows gives them the rest, by
/// their heights.
#[test]
fn a_tall_table_grows_its_rows() {
    let mut dom = doc(
        r#"<div><div class="t h4"><div class="r"><div class="c">a</div></div><div class="r"><div id="b" class="c">b</div></div></div></div>"#,
    );
    paint(&mut dom, &css(".h4 { height: 4 }"), 10, 5);
    assert_eq!((rect(&dom, "b").y, rect(&dom, "b").height), (2, 2));
}

/// CSS 2.1 §17.5.5: a `visibility: collapse` row takes no space, its
/// cells still sizing the columns; a collapsed column likewise.
#[test]
fn collapsed_rows_and_columns_take_no_space() {
    let mut dom = doc(
        r#"<div><div id="t" class="t"><div class="col"></div><div class="col x"></div><div class="r"><div class="c">a</div><div class="c">q</div></div><div class="r x"><div class="c">bbbbb</div><div class="c">q</div></div><div class="r"><div class="c">c</div><div class="c">q</div></div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(".x { visibility: collapse }"), 10, 4);
    assert_eq!(rows(&buf)[..2], ["a", "c"]);
    assert_eq!(rect(&dom, "t").width, 5);
}

// ── In other formatting contexts ───────────────────────────────────

/// CSS 2.1 §17.5.3 / §10.8.1: an `inline-table` is an atomic inline whose
/// baseline is its first row's.
#[test]
fn an_inline_table_sits_on_its_first_rows_baseline() {
    let mut dom = doc(
        r#"<div>ab <span class="it"><span class="r"><span class="c">X</span></span><span class="r"><span class="c">Y</span></span></span> cd</div>"#,
    );
    let buf = paint(&mut dom, &css(""), 20, 3);
    assert_eq!(rows(&buf)[..2], ["ab X cd", "   Y"]);
}

/// CSS Flexbox §4 / §9: a table is a flex item at its max-content width
/// in a row.
#[test]
fn a_table_is_a_flex_item() {
    let mut dom = doc(
        r#"<div class="fx"><div id="t" class="t"><div class="r"><div class="c">abc</div><div class="c">de</div></div></div><div>z</div></div>"#,
    );
    let buf = paint(&mut dom, &css(".fx { display: flex }"), 20, 2);
    assert_eq!(rows(&buf)[0], "abcdez");
    assert_eq!(rect(&dom, "t").width, 5);
}

/// A cell is a hit-test target; a column box, whose rect covers its
/// cells, is not (CSS 2.1 §17.5.1: columns only lend their background).
#[test]
fn cells_are_hit_and_columns_are_not() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="col"></div><div class="r"><div id="a" class="c">a</div></div></div></div>"#,
    );
    paint(&mut dom, &css(""), 10, 2);
    assert_eq!(dom.hit_test(0, 0), Some(by_id(&dom, "a")));
}

// ── The anonymous table around stray parts ─────────────────────────

/// CSS 2.1 §17.2.1 rule 3: table parts outside a table are wrapped in an
/// anonymous table, block-level in their parent's flow — consecutive
/// cells one row of it, the white space between them no box.
#[test]
fn stray_cells_are_wrapped_in_an_anonymous_table() {
    let mut dom = doc(
        "<div>before<div class=\"c\">a</div> <div class=\"c\">bb</div>\n<div class=\"c\">c</div>after</div>",
    );
    let buf = paint(&mut dom, &css(""), 20, 4);
    assert_eq!(rows(&buf)[..3], ["before", "abbc", "after"]);
}

/// §17.2.1 rule 3: stray rows share one anonymous table, so their cells
/// line up in its columns.
#[test]
fn stray_rows_share_one_anonymous_table() {
    let mut dom = doc(
        r#"<div><div class="r"><div class="c">a</div><div class="c">bbb</div></div><div class="r"><div class="c">cc</div><div class="c">d</div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(""), 20, 3);
    assert_eq!(rows(&buf)[..2], ["a bbb", "ccd"]);
}

/// The anonymous table's width is its parent's content width
/// (CSS Sizing 3 §5.1): an inline block around two stray cells is as wide
/// as the row, not as its widest cell.
#[test]
fn an_anonymous_table_sizes_its_parent() {
    let mut dom = doc(
        r#"<div><span class="ib"><span class="c">ab</span><span class="c">cd</span></span>|</div>"#,
    );
    let buf = paint(&mut dom, &css(".ib { display: inline-block }"), 20, 2);
    assert_eq!(rows(&buf)[0], "abcd|");
}

/// The caret and selection find text in cells — a cell's own lines and an
/// anonymous cell's (stored on its row) — from a point (CSSOM View
/// `caretPositionFromPoint`, `HitTestExt::position_at`).
#[test]
fn points_over_cells_resolve_to_their_text() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div id="a" class="c">ab</div>cd</div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(""), 10, 2);
    assert_eq!(rows(&buf)[0], "abcd");
    let a = dom.node(by_id(&dom, "a")).first_child().unwrap().id();
    let row = dom.node(by_id(&dom, "a")).parent_node().unwrap().id();
    let cd = dom.node(row).last_child().unwrap().id();
    let at = |x| dom.position_at(x, 0).map(|p| (p.node, p.offset));
    assert_eq!(at(1), Some((a, 1)));
    assert_eq!(at(3), Some((cd, 1)));
}
