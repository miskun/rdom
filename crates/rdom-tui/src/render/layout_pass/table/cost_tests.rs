//! C13-TFC: what one table layout measures — each cell's min- and
//! max-content widths and its height at its width walked once in a pass,
//! whatever the table's size (the pass's intrinsic memo serves the
//! repeats), and the table solved a bounded number of times.

use std::cell::Cell;

use super::SOLVES;
use crate::render::Rect;
use crate::render::layout_pass::intrinsic::memo_tests::{COLUMN_WALKS, ROW_WALKS};
use crate::{CascadeExt, LayoutExt, TuiDom};

/// A `display: table` of `rows` rows of three text cells under a block,
/// cascaded with `css` and laid out once: the table solves, and the Row
/// and Column content walks the pass made.
fn pass(css: &str, rows: usize) -> (usize, usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("div");
    dom.append_child(root, wrap).unwrap();
    let t = dom.create_element("div");
    dom.set_attribute(t, "class", "t").unwrap();
    dom.append_child(wrap, t).unwrap();
    for r in 0..rows {
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "r").unwrap();
        dom.append_child(t, row).unwrap();
        for c in 0..3 {
            let cell = dom.create_element("div");
            dom.set_attribute(cell, "class", "c").unwrap();
            dom.append_child(row, cell).unwrap();
            let text = dom.create_text_node(&format!("cell {r} {c} text"));
            dom.append_child(cell, text).unwrap();
        }
    }
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    for c in [&SOLVES, &ROW_WALKS, &COLUMN_WALKS] {
        c.with(|c| c.set(0));
    }
    dom.layout_dom(Rect::new(0, 0, 60, 80));
    (
        SOLVES.with(Cell::get),
        ROW_WALKS.with(Cell::get),
        COLUMN_WALKS.with(Cell::get),
    )
}

/// CSS 2.1 §17.5.2.2 / §17.5.3: the automatic algorithm reads each
/// cell's min- and max-content width and its height at its column's
/// width. The table is solved when its block parent measures its height
/// and when it lays out — a constant number of times — and the cells'
/// walks behind each solve are the memo's after the first: two Row walks
/// and one Column walk a cell (plus a few for the table and its parent),
/// at 3 rows and at 20.
#[test]
fn a_table_pass_measures_each_cell_a_bounded_number_of_times() {
    let css = ".t { display: table } .r { display: table-row } \
               .c { display: table-cell; padding: 0 1 }";
    for rows in [3, 20] {
        let cells = 3 * rows;
        let (solves, row_walks, column_walks) = pass(css, rows);
        assert!(solves <= 3, "{rows} rows: {solves} solves");
        assert!(
            row_walks <= 2 * cells + 4,
            "{rows} rows: {row_walks} Row walks"
        );
        assert!(
            column_walks <= cells + 4,
            "{rows} rows: {column_walks} Column walks"
        );
    }
}
