//! C13G-TABLE-TRACKS — `TuiAccessors::table_tracks`: a laid-out table's
//! used columns and rows (CSS 2.1 §17.5), as cell ranges from the table
//! box's content edge, the shape `grid_tracks` gives a grid — for a
//! header, a rule or a resize handle drawn outside the table.

use rdom_tui::TuiAccessors;

use super::{by_id, doc, paint, rows};

/// Track ranges as `(start, end)` pairs.
type Ranges = Vec<(i32, i32)>;

/// The tracks of `#t` in `markup`, laid out with `css` at 30 × 8, and the
/// painted rows.
fn tracks(markup: &str, css: &str) -> (Option<(Ranges, Ranges)>, Vec<String>) {
    let mut dom = doc(markup);
    let buf = paint(&mut dom, css, 30, 8);
    let t = dom.node(by_id(&dom, "t")).table_tracks();
    let pairs = |r: &[std::ops::Range<i32>]| r.iter().map(|r| (r.start, r.end)).collect();
    (t.map(|t| (pairs(t.columns()), pairs(t.rows()))), rows(&buf))
}

const TWO_BY_TWO: &str = r#"<div><table id="t"><tr><td>a</td><td>bbb</td></tr><tr><td>c</td><td>d</td></tr></table></div>"#;

/// §17.5: each column is the cells between its lines, from the table
/// box's content edge — a cell's border box in the separated model.
#[test]
fn a_tables_tracks_are_its_columns_and_rows() {
    let (t, _) = tracks(TWO_BY_TWO, "");
    let (columns, rows) = t.expect("a table");
    assert_eq!(columns, [(0, 3), (3, 8)]);
    assert_eq!(rows, [(0, 1), (1, 2)]);
}

/// §17.6.1: `border-spacing` is the gaps between the ranges and around
/// them; the table's border and padding are outside the content edge.
#[test]
fn spacing_is_between_the_ranges() {
    let (t, _) = tracks(
        TWO_BY_TWO,
        "table { border-spacing: 2 1; border: solid; padding: 0 1 }",
    );
    let (columns, rows) = t.expect("a table");
    assert_eq!(columns, [(2, 5), (7, 12)]);
    assert_eq!(rows, [(1, 2), (3, 4)]);
}

/// §17.6.2: in the collapsing model the table has no padding and its
/// border is the grid's outer line — the content edge is the border
/// box's — and each one-cell line is outside the ranges.
#[test]
fn collapsed_lines_are_between_the_ranges() {
    let (t, _) = tracks(
        TWO_BY_TWO,
        "table { border-collapse: collapse } td { border: solid }",
    );
    let (columns, rows) = t.expect("a table");
    assert_eq!(columns, [(1, 4), (5, 10)]);
    assert_eq!(rows, [(1, 2), (3, 4)]);
}

/// §17.4: a caption is outside the table box, so the rows count from
/// below it.
#[test]
fn captions_are_outside_the_tracks() {
    let markup = r#"<div><table id="t"><caption>Cap</caption><tr><td>a</td></tr></table></div>"#;
    let (t, _) = tracks(markup, "");
    assert_eq!(t.expect("a table").1, [(0, 1)]);
}

/// CSS 2.1 §17.5 ("the direction of the table"), Writing Modes 4 §2.1:
/// an `rtl` table's first column is its rightmost — in the paint and in
/// the tracks, which stay in column order, as an `rtl` grid's do.
#[test]
fn an_rtl_tables_first_column_is_on_the_right() {
    let (t, painted) = tracks(TWO_BY_TWO, "table { direction: rtl }");
    assert_eq!(t.expect("a table").0, [(5, 8), (0, 5)]);
    assert_eq!(painted[0], " bbb  a");
    let (t, painted) = tracks(
        TWO_BY_TWO,
        "table { direction: rtl; border-collapse: collapse } td { border: solid } \
         td:first-child { border-right-style: double }",
    );
    assert_eq!(t.expect("a table").0, [(7, 10), (1, 6)]);
    assert_eq!(painted[1], "│ bbb │ a ║");
    // §17.6.2: a cell's right border is on the line before its column.
    let (t, _) = tracks(
        TWO_BY_TWO,
        "table { direction: rtl; border-collapse: collapse } td { border-right: solid }",
    );
    assert_eq!(t.expect("a table").0, [(6, 9), (0, 5)]);
}

/// `visibility: collapse` (§17.5.5): a collapsed column is an empty range
/// where it would have been.
#[test]
fn a_collapsed_column_is_an_empty_range() {
    let markup = r#"<div><table id="t"><colgroup><col><col class="x"></colgroup><tr><td>a</td><td>bbb</td><td>c</td></tr></table></div>"#;
    let (t, _) = tracks(markup, ".x { visibility: collapse }");
    assert_eq!(t.expect("a table").0, [(0, 3), (3, 3), (3, 6)]);
}

/// A box that is not a table, and a table never laid out, have none.
#[test]
fn a_block_has_no_table_tracks() {
    let (t, _) = tracks(r#"<div id="t">x</div>"#, "");
    assert_eq!(t, None);
    let dom = doc(r#"<table id="t"><tr><td>a</td></tr></table>"#);
    assert!(dom.node(by_id(&dom, "t")).table_tracks().is_none());
}
