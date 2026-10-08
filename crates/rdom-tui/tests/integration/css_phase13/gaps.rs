//! The spec gaps the Phase 13 gate found (C13G-SPEC-GAPS; CSS 2.1 §17,
//! CSS Tables 3): fixed layout's extra width, percentages past 100%, the
//! excess over percent columns, a rowspanning `baseline` cell's rows, and
//! a table part misparented in an anonymous cell.

use rdom_tui::{LayoutRect, TuiNodeExt};

use super::{by_id, doc, paint, rows};

const PARTS: &str = ".t { display: table } .r { display: table-row } \
    .c { display: table-cell } .col { display: table-column }";

fn css(extra: &str) -> String {
    format!("{PARTS} {extra}")
}

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

fn widths(dom: &rdom_tui::TuiDom, ids: &[&str]) -> Vec<u16> {
    ids.iter().map(|id| rect(dom, id).width).collect()
}

/// CSS 2.1 §17.5.2.1: in fixed layout, "if the table is wider than the
/// columns, the extra space should be distributed over the columns" —
/// every column fixed, the table's `width` is still the grid's (CSS Tables
/// 3: by the columns' widths, equally when they are all 0).
#[test]
fn fixed_layout_spreads_the_extra_width_over_fixed_columns() {
    let mut dom = doc(
        r#"<div><div id="t" class="t"><div class="col"></div><div class="col w3"></div><div class="r"><div id="a" class="c">a</div><div id="b" class="c">b</div></div></div></div>"#,
    );
    paint(
        &mut dom,
        &css(".t { table-layout: fixed; width: 20 } .col { width: 5 } .w3 { width: 15 }"),
        30,
        3,
    );
    assert_eq!(rect(&dom, "t").width, 20);
    assert_eq!(widths(&dom, &["a", "b"]), [5, 15]);
    paint(
        &mut dom,
        &css(".t { table-layout: fixed; width: 40 } .col { width: 5 } .w3 { width: 15 }"),
        50,
        3,
    );
    assert_eq!(widths(&dom, &["a", "b"]), [10, 30]);
}

/// CSS Tables 3 ("intrinsic percentage width of a column"): a column's
/// percentage is at most what the columns before it leave of 100% — the
/// second `80%` column is a `20%` one — so the percentages never claim
/// more than the table.
#[test]
fn percentages_past_100_are_clamped_in_column_order() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div id="a" class="c p">a</div><div id="b" class="c p">b</div><div id="c" class="c">c</div></div></div></div>"#,
    );
    paint(&mut dom, &css(".t { width: 21 } .p { width: 80% }"), 30, 3);
    assert_eq!(widths(&dom, &["a", "b", "c"]), [16, 4, 1]);
}

/// CSS Tables 3 ("distributing excess width to columns"): with only
/// percent columns, the width past their percentages goes to them in
/// proportion to their percentages, not their content.
#[test]
fn excess_width_goes_to_percent_columns_by_percentage() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div id="a" class="c p10">a</div><div id="b" class="c p20">b</div></div></div></div>"#,
    );
    paint(
        &mut dom,
        &css(".t { width: 30 } .p10 { width: 10% } .p20 { width: 20% }"),
        40,
        3,
    );
    assert_eq!(widths(&dom, &["a", "b"]), [10, 20]);
}

/// CSS 2.1 §17.5.3: a `baseline` cell spanning rows is aligned on its
/// first row's baseline, and the rows it spans are tall enough to hold it
/// moved down there.
#[test]
fn a_rowspanning_baseline_cell_gets_the_rows_it_needs() {
    let mut dom = doc(
        r#"<div><table id="t"><tr><td class="pad">A</td><td rowspan="2"><div id="x">x</div><div>y</div><div>z</div></td></tr><tr><td>B</td></tr></table></div>"#,
    );
    paint(
        &mut dom,
        "td { vertical-align: baseline } .pad { padding-top: 2 }",
        20,
        8,
    );
    assert_eq!(rect(&dom, "x").y, 2, "x on A's baseline");
    assert_eq!(rect(&dom, "t").height, 5);
}

/// CSS 2.1 §17.2.1 rule 3.2: table parts misparented in an anonymous
/// cell — rows inside a row — get one anonymous table around them, so
/// their cells share its columns (each row laid out as a block of its own
/// would size its columns alone).
#[test]
fn rows_inside_a_row_share_an_anonymous_table() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="r"><div class="c">a</div><div class="c">b</div></div><div class="r"><div class="c">ccc</div><div class="c">d</div></div></div></div></div>"#,
    );
    let buf = paint(&mut dom, &css(""), 10, 3);
    assert_eq!(rows(&buf)[..2], ["a  b", "cccd"]);
}
