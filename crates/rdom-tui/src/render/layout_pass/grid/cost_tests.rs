//! C7-GRID-CORE: what one grid layout measures — each item's
//! contributions measured at most once per sizing run, the runs a pass
//! makes bounded, and the content walks behind them served by the pass's
//! intrinsic memo (`intrinsic::memo`), not a second one.

use std::cell::Cell;

use super::Dimension;
use super::contribution::MEASURES;
use super::size::RUNS;
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

/// The axes each `size_grid` call of one layout of `css` (a grid `.g`
/// holding an empty `.r` and a `.t` of text) sized, in order.
fn runs_per_call(css: &str) -> Vec<Vec<Dimension>> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = dom.create_element("div");
    dom.set_attribute(g, "class", "g").unwrap();
    dom.append_child(root, g).unwrap();
    let mut item = |class: &str| {
        let e = dom.create_element("div");
        dom.set_attribute(e, "class", class).unwrap();
        dom.append_child(g, e).unwrap();
        e
    };
    item("r");
    let t = item("t");
    let text = dom.create_text_node("aaaa bbbb cccc dddd eeee");
    dom.append_child(t, text).unwrap();
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    RUNS.with(|r| r.borrow_mut().clear());
    dom.layout_dom(Rect::new(0, 0, 20, 12));
    let mut calls: Vec<Vec<Dimension>> = Vec::new();
    for run in RUNS.with(|r| r.borrow().clone()) {
        match run {
            None => calls.push(Vec::new()),
            Some(d) => calls.last_mut().expect("a run inside a call").push(d),
        }
    }
    calls
}

/// CSS Grid 2 §11.1 steps 3–4 re-resolve the columns, then the rows,
/// "once only": a grid whose ratio item's width changes with its row
/// sizes each axis twice in the calls that size its rows — columns,
/// rows, columns, rows — and never more; one with nothing to re-resolve
/// sizes each axis once a call.
#[test]
fn re_resolution_is_bounded_to_once_an_axis() {
    use Dimension::{Columns as C, Rows as R};
    let calls = runs_per_call(
        ".g { display: grid; grid-template-columns: auto 1fr } \
         .r { height: 100%; aspect-ratio: 5 }",
    );
    assert!(calls.iter().any(|c| c == &[C, R, C, R]), "{calls:?}");
    assert!(
        calls
            .iter()
            .all(|c| c == &[C] || c == &[C, R] || c == &[C, R, C, R]),
        "{calls:?}"
    );
    let calls = runs_per_call(".g { display: grid; grid-template-columns: auto 1fr }");
    assert!(calls.iter().all(|c| c == &[C] || c == &[C, R]), "{calls:?}");
}

/// How many `size_grid` calls one layout pass makes for a grid holding a
/// chain of `depth` column subgrids, each the next's parent, the
/// innermost holding two text items; `.g` styled `g` as well.
fn nested_subgrid_calls(depth: usize, g_css: &str) -> usize {
    chain_calls(
        depth,
        &format!(
            ".g {{ display: grid; grid-template-columns: auto auto; {g_css} }} \
             .s {{ grid-column: 1 / 3; display: grid; grid-template-columns: subgrid }}"
        ),
    )
}

/// [`nested_subgrid_calls`] with the sheet `css` for `.g` and the `.s`
/// chain, every second `.s` of which is a `.t` too.
fn chain_calls(depth: usize, css: &str) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = dom.create_element("div");
    dom.set_attribute(g, "class", "g").unwrap();
    dom.append_child(root, g).unwrap();
    let mut parent = g;
    for k in 0..depth {
        let s = dom.create_element("div");
        dom.set_attribute(s, "class", if k % 2 == 0 { "s" } else { "s t" })
            .unwrap();
        dom.append_child(parent, s).unwrap();
        parent = s;
    }
    for text in ["aa", "bbb"] {
        let e = dom.create_element("div");
        dom.append_child(parent, e).unwrap();
        let t = dom.create_text_node(text);
        dom.append_child(e, t).unwrap();
    }
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    RUNS.with(|r| r.borrow_mut().clear());
    super::places::PLACES.with(|n| n.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 20));
    RUNS.with(|r| r.borrow().iter().filter(|run| run.is_none()).count())
}

/// The `place_grid` calls of [`nested_subgrid_calls`]`(depth, g_css)`'s
/// pass.
fn nested_subgrid_places(depth: usize, g_css: &str) -> usize {
    nested_subgrid_calls(depth, g_css);
    super::places::PLACES.with(std::cell::Cell::get)
}

/// CSS Grid 2 §9.5: a subgrid's items size its parent's tracks, and a
/// column subgrid's height is its rows sized at its parent's columns —
/// measured by sizing it, which measures its own subgrids the same way.
/// The recursion is bounded: each subgrid of a chain is sized once to be
/// measured (the pass memoizes the measurement, so its parent's own
/// layout does not measure it again) and once to be laid out — 2 + 2 ×
/// depth calls in all, where unmemoized measurements made the count
/// grow with the square of the depth.
#[test]
fn nested_subgrids_are_sized_a_bounded_number_of_times() {
    for depth in 0..7 {
        assert_eq!(
            nested_subgrid_calls(depth, ""),
            2 + 2 * depth,
            "depth {depth}"
        );
    }
}

/// C7G-MEMO-PURITY — CSS Grid 2 §10.4 / §11.5 step 1: a subgrid is
/// stretched, so in no baseline-sharing group, and a baseline-aligned
/// parent does not measure it for a shim (a measurement with tracks the
/// parent has not laid out yet, thrown away): `align-items: baseline`
/// sizes no more grids than `start`, which measures the subgrid's height
/// once as it arranges it, as `baseline` does.
#[test]
fn a_baseline_grid_measures_no_shim_for_its_subgrid() {
    assert_eq!(
        nested_subgrid_calls(1, "align-items: baseline"),
        nested_subgrid_calls(1, "align-items: start")
    );
}

/// C7G-SUBGRID-COST — CSS Grid 2 §9.5: each subgrid of a chain is placed
/// and sized a bounded number of times a pass, so both counts grow by a
/// constant with each level. The chain alternates the axis it subgrids —
/// a rows subgrid whose child subgrids its own columns, whose child
/// subgrids its rows, … — so a flattened level's items include a subgrid
/// on the other axis, measured with that axis inherited (`size::
/// measure_subgrid`, memoized for the pass). Measured as a plain grid
/// instead, outside the memo since C7G-MEMO-PURITY, each such level was
/// sized again for every sizing run above it (7, 14, 39, 61, 161, 228,
/// 598 sizing calls at depths 1 to 7).
#[test]
fn nested_subgrids_are_placed_a_bounded_number_of_times() {
    let linear = |counts: &[usize]| {
        let steps: Vec<usize> = counts.windows(2).map(|w| w[1] - w[0]).collect();
        steps.windows(2).all(|w| w[0] == w[1])
    };
    let places: Vec<usize> = (0..8).map(|d| nested_subgrid_places(d, "")).collect();
    assert!(linear(&places), "column subgrids: {places:?} placements");
    let css = ".g { display: grid; grid-template-columns: auto auto; \
               grid-template-rows: auto auto } \
               .s { grid-area: 1 / 1 / 3 / 3; display: grid; \
               grid-template-rows: subgrid; grid-template-columns: auto auto } \
               .t { grid-template-rows: auto auto; grid-template-columns: subgrid }";
    let (mut sizes, mut places) = (Vec::new(), Vec::new());
    for depth in 1..8 {
        sizes.push(chain_calls(depth, css));
        places.push(super::places::PLACES.with(std::cell::Cell::get));
    }
    // From depth 1 the levels alternate, each pair adding the same work.
    let pairs = |v: &[usize]| v.iter().step_by(2).copied().collect::<Vec<_>>();
    assert!(
        linear(&pairs(&sizes)),
        "alternating: {sizes:?} sizing calls"
    );
    assert!(
        linear(&pairs(&places)),
        "alternating: {places:?} placements"
    );
}

/// C7G-SUBGRID-COST — a grid's line names (CSS Grid 2 §7.2) are copied
/// into its kept lines (`TuiExt::grid_lines`, §9.1) only when it is laid
/// out: measuring it (§5.2) allocates, for three named lines holding six
/// names, only the three lists placement (§8.3) reads them from — each
/// name borrowed from the style, none copied.
#[test]
fn measuring_a_grid_copies_no_line_names() {
    use crate::layout::Direction;
    use crate::render::layout_pass::intrinsic::intrinsic_size;
    use crate::test_alloc::allocations_in;
    let cost = |columns: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let g = dom.create_element("div");
        dom.set_attribute(g, "class", "g").unwrap();
        dom.append_child(root, g).unwrap();
        for text in ["aa", "bbb"] {
            let e = dom.create_element("div");
            dom.append_child(g, e).unwrap();
            let t = dom.create_text_node(text);
            dom.append_child(e, t).unwrap();
        }
        let css = format!(".g {{ display: grid; grid-template-columns: {columns} }}");
        let sheet = rdom_css::from_css_strict(&css).expect("sheet parses");
        dom.cascade(&sheet);
        // Outside a pass nothing is memoized: each call measures.
        intrinsic_size(&dom, g, Direction::Column, 20, 20);
        allocations_in(|| {
            intrinsic_size(&dom, g, Direction::Column, 20, 20);
        })
    };
    assert_eq!(cost("[a b] 2 [c] 3 [d e f]"), cost("2 3") + 3);
}
