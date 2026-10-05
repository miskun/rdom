//! C5G-PERF-AND-TESTS: the min- / max-content sizes a layout pass
//! measures are memoized per node, so nested intrinsic-keyword boxes
//! (CSS Sizing 3 §3.1 `fit-content`) cost one walk of each subtree per
//! measurement, not one per enclosing keyword box.

use std::cell::Cell;

use crate::render::Rect;
use crate::{CascadeExt, LayoutExt, TuiDom};

thread_local! {
    /// Row-axis content measurements computed (not served from the memo).
    pub(super) static ROW_WALKS: Cell<usize> = const { Cell::new(0) };
    /// Column-axis content measurements computed (not served from the memo).
    pub(super) static COLUMN_WALKS: Cell<usize> = const { Cell::new(0) };
}

/// `depth` nested `div.f`s around a text leaf, laid out once; the Row
/// content measurements that pass computed.
fn walks(depth: usize) -> usize {
    let mut dom = TuiDom::new();
    let mut parent = dom.root();
    for _ in 0..depth {
        let div = dom.create_element("div");
        dom.set_attribute(div, "class", "f").unwrap();
        dom.append_child(parent, div).unwrap();
        parent = div;
    }
    let text = dom.create_text_node("one two three");
    dom.append_child(parent, text).unwrap();
    let sheet = rdom_style::Stylesheet::new()
        .rule(
            ".f",
            rdom_style::TuiStyle::new()
                .width(crate::layout::IntrinsicSize::FitContent)
                .padding(crate::layout::Padding::new(0, 1, 0, 1)),
        )
        .unwrap();
    dom.cascade(&sheet);
    ROW_WALKS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 60, 20));
    ROW_WALKS.with(Cell::get)
}

/// Each box is measured a bounded number of times however deep the
/// nesting — its min- and max-content sizes for each containing-block
/// width it is asked about (its own layout's, and 0 inside an ancestor's
/// measurement) — so the count grows linearly with the depth, not with
/// its square (unmemoized: 48 walks at 4 levels, 780 at 16).
#[test]
fn nested_fit_content_boxes_measure_each_subtree_once() {
    for depth in [4, 16] {
        let n = walks(depth);
        assert!(n <= 6 * (depth + 1), "{depth} levels: {n} walks");
    }
}

/// `depth` nested elements of class `class`, each holding a word and the
/// next (the innermost a word), in a `p`, styled by `style`; laid out
/// once, the Column content measurements that pass computed.
fn column_walks(depth: usize, style: rdom_style::TuiStyle) -> usize {
    let mut dom = TuiDom::new();
    let p = dom.create_element("p");
    let root = dom.root();
    dom.append_child(root, p).unwrap();
    let mut parent = p;
    for _ in 0..depth {
        let text = dom.create_text_node("ab ");
        dom.append_child(parent, text).unwrap();
        let span = dom.create_element("span");
        dom.set_attribute(span, "class", "n").unwrap();
        dom.append_child(parent, span).unwrap();
        parent = span;
    }
    let text = dom.create_text_node("cd");
    dom.append_child(parent, text).unwrap();
    let sheet = rdom_style::Stylesheet::new().rule(".n", style).unwrap();
    dom.cascade(&sheet);
    COLUMN_WALKS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 60, 20));
    COLUMN_WALKS.with(Cell::get)
}

/// C6G-ATOM-COST: an inline block's rows in its line (CSS 2.1 §10.8) —
/// its height and its baseline (§10.8.1) — come from measurements of its
/// subtree on the Column axis, which pack its own atoms, so unmemoized
/// they cost a walk per level per enclosing level (81 at 8 levels).
/// Memoized for the pass, each subtree is measured once per width it is
/// asked about — here two: its own width in its line, and the
/// containing block's, where the enclosing box's block-flow estimate
/// stacks it — so the count grows linearly with the depth.
#[test]
fn nested_inline_blocks_measure_each_subtree_once() {
    let style = || {
        rdom_style::TuiStyle::new()
            .display(crate::layout::Display::InlineBlock)
            .padding(crate::layout::Padding::new(0, 1, 0, 1))
    };
    for depth in [4, 12] {
        let n = column_walks(depth, style());
        assert!(n <= 2 * (depth + 1), "{depth} levels: {n} walks");
    }
}

/// C6G-ATOM-COST for flex baseline alignment (CSS Flexbox §8.3): an
/// item's baseline box measures its height and its content rows, each a
/// Column measurement of its subtree; nested baseline-aligned rows
/// measure each subtree once, not once per enclosing row (exponential:
/// 11469 walks at 8 levels).
#[test]
fn nested_baseline_rows_measure_each_subtree_once() {
    let style = || {
        rdom_style::TuiStyle::new()
            .display(crate::layout::Display::Block)
            .flow(crate::layout::Flow::Flex)
            .align_items(crate::layout::Align::Baseline.into())
    };
    for depth in [4, 12] {
        let n = column_walks(depth, style());
        assert!(n <= depth + 1, "{depth} levels: {n} walks");
    }
}
