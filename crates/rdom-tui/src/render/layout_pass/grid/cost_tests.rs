//! C7-GRID-CORE: what one grid layout measures — each item's
//! contributions measured at most once per sizing run, the runs a pass
//! makes bounded, and the content walks behind them served by the pass's
//! intrinsic memo (`intrinsic::memo`), not a second one.

use std::cell::Cell;

use super::contribution::MEASURES;
use crate::render::Rect;
use crate::render::layout_pass::intrinsic::memo_tests::{COLUMN_WALKS, ROW_WALKS};
use crate::{CascadeExt, LayoutExt, TuiDom};

/// A grid `.g` under a block, holding `n` text items, cascaded with
/// `css` and laid out once; the contributions measured, and the Row and
/// Column content walks the pass made.
fn pass(css: &str, n: usize) -> (usize, usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("div");
    dom.append_child(root, wrap).unwrap();
    let g = dom.create_element("div");
    dom.set_attribute(g, "class", "g").unwrap();
    dom.append_child(wrap, g).unwrap();
    for k in 0..n {
        let s = dom.create_element("span");
        dom.append_child(g, s).unwrap();
        let t = dom.create_text_node(&format!("item {k} text"));
        dom.append_child(s, t).unwrap();
    }
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    for c in [&MEASURES, &ROW_WALKS, &COLUMN_WALKS] {
        c.with(|c| c.set(0));
    }
    dom.layout_dom(Rect::new(0, 0, 60, 30));
    (
        MEASURES.with(Cell::get),
        ROW_WALKS.with(Cell::get),
        COLUMN_WALKS.with(Cell::get),
    )
}

/// CSS Grid 2 §11.5: an `auto` column sizes to its items' minimum and
/// max-content contributions, and an `auto` row of an `auto`-height grid
/// — sized to its content, as under a max-content constraint — to their
/// limited min-content (min-content and minimum) and max-content ones:
/// five measurements an item in a sizing run, each once, the run caching
/// them. A pass sizes the grid twice: when its block parent measures its
/// height (§5.2), and when it lays out (§11.1). Behind them, each item's
/// min- and max-content width and its min- and max-content height at its
/// column's width are walked once in the pass: the second run reads the
/// intrinsic memo.
#[test]
fn a_grid_pass_measures_each_item_a_bounded_number_of_times() {
    for n in [3, 12] {
        let (measures, rows, columns) = pass(
            ".g { display: grid; grid-template-columns: auto auto auto }",
            n,
        );
        assert_eq!(measures, 10 * n, "{n} items: contributions measured");
        assert!(rows <= 2 * n, "{n} items: {rows} Row walks");
        // And the grid's own height and its parent's.
        assert!(columns <= 2 * n + 2, "{n} items: {columns} Column walks");
    }
}

/// CSS Grid 2 §11.5 step 4 / §11.7: in `1fr` rows of an `auto`-height
/// grid an item's minimum and max-content contributions are each asked
/// twice in a run — by the intrinsic minimums and the max-content
/// minimums, and again by the flex fraction — and measured once: the run
/// caches them, so the count is the same five an item a run.
#[test]
fn a_sizing_run_measures_a_contribution_once_however_often_it_is_read() {
    let (measures, _, _) = pass(
        ".g { display: grid; grid-template-columns: auto auto auto; \
         grid-template-rows: repeat(4, 1fr) }",
        12,
    );
    assert_eq!(measures, 10 * 12);
}
