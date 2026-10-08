//! C13-TFC: what one table layout measures — each cell's min- and
//! max-content widths and its height at its width walked once in a pass,
//! whatever the table's size (the pass's intrinsic memo serves the
//! repeats), and the table solved a bounded number of times.

use std::cell::Cell;

use super::{ANONYMOUS_PACKS, COLUMN_SCANS, GROUP_SCANS, SOLVES, STRUCTURES};
use crate::node::TuiNodeExt;
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

/// An HTML table of `groups` `<colgroup span=1000>`s (a collapsed border
/// on each) and one row of `cells` `<td colspan=1000>`s, laid out once.
fn wide_table(groups: usize, cells: usize) -> (TuiDom, Vec<rdom_core::NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let t = dom.create_element("table");
    dom.append_child(root, t).unwrap();
    for _ in 0..groups {
        let g = dom.create_element("colgroup");
        dom.set_attribute(g, "span", "1000").unwrap();
        dom.append_child(t, g).unwrap();
    }
    let tr = dom.create_element("tr");
    dom.append_child(t, tr).unwrap();
    let mut tds = Vec::new();
    for i in 0..cells {
        let td = dom.create_element("td");
        dom.set_attribute(td, "colspan", "1000").unwrap();
        dom.append_child(tr, td).unwrap();
        let text = dom.create_text_node(&format!("{i}"));
        dom.append_child(td, text).unwrap();
        tds.push(td);
    }
    let sheet = rdom_css::from_css_strict(
        "table { display: table; border-collapse: collapse } \
         colgroup { display: table-column-group; border: solid } \
         tr { display: table-row } td { display: table-cell }",
    )
    .expect("sheet parses");
    dom.cascade(&sheet);
    (dom, tds)
}

/// A hostile `span` costs linear time (C13G-SPAN-COST): twenty
/// `<colgroup span=1000>`s make 20 000 columns (HTML §4.9.3 caps each
/// span at 1000, not their sum), and a collapsed table marks each column
/// box's border lines and places each box from its column range — found
/// once, when the structure is built, not by scanning every column for
/// every column (~8·10⁸ comparisons a pass).
#[test]
fn hostile_column_spans_cost_linear_time() {
    let (mut dom, _) = wide_table(20, 1);
    COLUMN_SCANS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    let scans = COLUMN_SCANS.with(Cell::get);
    assert!(
        scans <= 4 * 20_000,
        "{scans} column scans for 20 000 columns"
    );
}

/// The grid is capped at 65 535 columns (C13G-SPAN-COST): no terminal
/// cell offset reaches past `u16::MAX`, so a column there could never
/// show. A cell reaching past the cap is cut at it; one starting past it
/// has no box (an empty rect), as a cell in a
/// collapsed column (§17.5.5).
#[test]
fn the_grid_is_capped_at_u16_max_columns() {
    let (mut dom, tds) = wide_table(0, 70);
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    let solved = {
        let t = dom.query_selector("table").unwrap().id();
        let c = dom.node(t).computed_rc().unwrap();
        super::solve(&dom, super::TableBox::Element(t), &c, 60, 60)
    };
    assert_eq!(solved.skeleton.grid.columns, usize::from(u16::MAX));
    let cut = solved
        .skeleton
        .grid
        .cells
        .iter()
        .find(|c| c.column == 65_000)
        .unwrap();
    assert_eq!(cut.columns, 535);
    assert!(
        solved
            .skeleton
            .grid
            .cells
            .iter()
            .all(|c| c.column_end() <= usize::from(u16::MAX))
    );
    let past = dom.node(tds[66]).layout_rect().unwrap();
    assert_eq!((past.width, past.height), (0, 0));
}

/// A `display: table` of `rows` rows, each in its own row group (as a
/// generator emitting one `<tbody>` per row does), each row two text
/// cells and a run of loose text — an anonymous cell (§17.2.1 rule 2.3)
/// — under a block, cascaded.
fn long_table(rows: usize) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("div");
    dom.append_child(root, wrap).unwrap();
    let t = dom.create_element("div");
    dom.set_attribute(t, "class", "t").unwrap();
    dom.append_child(wrap, t).unwrap();
    for r in 0..rows {
        let group = dom.create_element("div");
        dom.set_attribute(group, "class", "g").unwrap();
        dom.append_child(t, group).unwrap();
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "r").unwrap();
        dom.append_child(group, row).unwrap();
        for c in 0..2 {
            let cell = dom.create_element("div");
            dom.set_attribute(cell, "class", "c").unwrap();
            dom.append_child(row, cell).unwrap();
            let text = dom.create_text_node(&format!("cell {r} {c}"));
            dom.append_child(cell, text).unwrap();
        }
        let loose = dom.create_text_node(&format!("loose {r} text"));
        dom.append_child(row, loose).unwrap();
    }
    let sheet = rdom_css::from_css_strict(
        ".t { display: table; border-collapse: collapse } \
         .g { display: table-row-group; border-top: solid } \
         .r { display: table-row } .c { display: table-cell; padding: 0 1 }",
    )
    .expect("sheet parses");
    dom.cascade(&sheet);
    dom
}

/// C13G-TABLE-COST — a table's work in one layout pass is linear in its
/// size: its structure, grid and lines are built once a pass (they are
/// pure while it runs), each row group's rows found once (not by
/// scanning every row for every group: a generator emitting a `<tbody>`
/// per row made that quadratic), each anonymous cell's runs packed a
/// bounded number of times (its measures memoized with the pass, as an
/// element's are), each element cell measured a bounded number of times —
/// at 1000 rows as at 10.
#[test]
fn a_long_table_costs_linear_work_a_pass() {
    for rows in [10, 1000] {
        let mut dom = long_table(rows);
        for c in [
            &SOLVES,
            &STRUCTURES,
            &GROUP_SCANS,
            &ANONYMOUS_PACKS,
            &ROW_WALKS,
            &COLUMN_WALKS,
        ] {
            c.with(|c| c.set(0));
        }
        dom.layout_dom(Rect::new(0, 0, 60, 80));
        let get = |c: &'static std::thread::LocalKey<Cell<usize>>| c.with(Cell::get);
        let cells = 2 * rows;
        eprintln!(
            "{rows} rows: {} solves, {} structures, {} group scans, {} anonymous packs, \
             {} Row walks, {} Column walks",
            get(&SOLVES),
            get(&STRUCTURES),
            get(&GROUP_SCANS),
            get(&ANONYMOUS_PACKS),
            get(&ROW_WALKS),
            get(&COLUMN_WALKS)
        );
        assert_eq!(get(&STRUCTURES), 1, "{rows} rows: structures");
        assert!(
            get(&GROUP_SCANS) <= 4 * rows,
            "{rows} rows: {} group scans",
            get(&GROUP_SCANS)
        );
        assert!(
            get(&ANONYMOUS_PACKS) <= 4 * rows,
            "{rows} rows: {} anonymous packs",
            get(&ANONYMOUS_PACKS)
        );
        assert!(
            get(&ROW_WALKS) <= 2 * cells + 4,
            "{rows} rows: {} Row walks",
            get(&ROW_WALKS)
        );
        assert!(
            get(&COLUMN_WALKS) <= cells + 4,
            "{rows} rows: {} Column walks",
            get(&COLUMN_WALKS)
        );
    }
}

/// C13G-TABLE-COST — the allocations of a table's layout pass, per row,
/// are pinned: a pass that rebuilds the structure, or copies it per
/// solve, shows here.
#[test]
fn a_long_table_allocates_a_bounded_amount_per_row() {
    use crate::test_alloc::allocations_in;
    let per_row = |rows: usize| {
        let mut dom = long_table(rows);
        dom.layout_dom(Rect::new(0, 0, 60, 80));
        allocations_in(|| dom.layout_dom(Rect::new(0, 0, 60, 80))) / rows as u64
    };
    let (small, large) = (per_row(100), per_row(1000));
    eprintln!("allocations a row: {small} at 100 rows, {large} at 1000");
    assert!(large <= ALLOCATIONS_PER_ROW, "{large} allocations a row");
}

/// The allocations one row of `long_table` costs a layout pass, measured
/// when C13G-TABLE-COST landed (with a little room).
const ALLOCATIONS_PER_ROW: u64 = 230;

/// C13G-TABLE-TRACKS: `table_tracks()` reads what the last layout kept —
/// no structure is built and nothing is solved to answer it.
#[test]
fn table_tracks_read_the_kept_layout_without_solving() {
    use crate::TuiAccessors;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let t = dom.create_element("table");
    dom.append_child(root, t).unwrap();
    let tr = dom.create_element("tr");
    dom.append_child(t, tr).unwrap();
    for text in ["a", "bbb"] {
        let td = dom.create_element("td");
        dom.append_child(tr, td).unwrap();
        let x = dom.create_text_node(text);
        dom.append_child(td, x).unwrap();
    }
    dom.cascade(&crate::Stylesheet::new());
    dom.layout_dom(Rect::new(0, 0, 20, 4));
    for c in [&SOLVES, &STRUCTURES] {
        c.with(|c| c.set(0));
    }
    let tracks = dom.node(t).table_tracks().expect("a laid-out table");
    assert_eq!(tracks.columns(), [0..3, 3..8]);
    assert_eq!((SOLVES.with(Cell::get), STRUCTURES.with(Cell::get)), (0, 0));
}
