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
