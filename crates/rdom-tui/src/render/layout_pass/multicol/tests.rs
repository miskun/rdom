//! Multi-column layout's costs (C15-COLUMNS): a page without a
//! multi-column container lays none out, and one with a container lays it
//! out once per layout pass — its content in one tall column, fragmented
//! without laying it out again.

use crate::prelude::*;
use crate::render::layout_pass::fragment::BREAKER_RUNS;
use crate::render::layout_pass::multicol::MULTICOL_LAYOUTS;

/// `n` paragraphs of one line under a `<div>` styled `css`, laid out in a
/// 40 × 10 viewport: the multi-column layouts and breaker runs the pass
/// made, and the elements it laid out.
fn layout_costs(n: usize, css: &str) -> (usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    for _ in 0..n {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(div, p).unwrap();
    }
    let sheet = rdom_css::from_css_strict(&format!("p {{ margin: 0 }} {css}")).unwrap();
    dom.cascade(&sheet);
    MULTICOL_LAYOUTS.with(|c| c.set(0));
    BREAKER_RUNS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    (
        MULTICOL_LAYOUTS.with(|c| c.get()),
        BREAKER_RUNS.with(|c| c.get()),
    )
}

/// No multi-column container, no multi-column layout and no breaker run;
/// one container balancing 64 lines over 4 columns: one layout, and the
/// breaker runs a binary search over the 64 rows costs (`⌈log₂ 64⌉ + 2`).
#[test]
fn only_a_multicol_container_pays_for_columns() {
    assert_eq!(layout_costs(64, ""), (0, 0));
    let (layouts, runs) = layout_costs(64, "div { column-count: 4 }");
    assert_eq!(layouts, 1);
    assert!(runs <= 8, "{runs} breaker runs");
}

/// Architect N10 (C15G-MULTICOL-COST): a column narrower than a cell holds
/// nothing, so `column-count` is capped at the columns of one cell that fit
/// (CSS Multi-column 1 §3.4's `W` is never below a cell) — `column-count:
/// 65535` in 40 cells with a one-cell gap is 20 columns, not 65 535 column
/// boxes per layout.
#[test]
fn the_column_count_is_capped_at_one_cell_columns() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let t = dom.create_text_node("x");
    dom.append_child(div, t).unwrap();
    let sheet = rdom_css::from_css_strict("div { column-count: 65535; column-gap: 1 }").unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    let computed = dom.node(div).computed().cloned().unwrap();
    let cols = super::geometry::columns(&computed, 40);
    assert_eq!((cols.count, cols.width), (20, 1));
    let boxes = match dom.node(div).ext().and_then(|e| e.kept.as_deref()) {
        Some(crate::ext::KeptLayout::Columns(sets)) => sets.iter().map(|s| s.columns.len()).sum(),
        _ => 0,
    };
    assert_eq!(boxes, 20, "column boxes kept");
}
