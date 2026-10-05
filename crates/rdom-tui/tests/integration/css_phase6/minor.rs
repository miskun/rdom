//! C6G-MINOR — small fixes from the Phase 6 architect gate: a scroll
//! container's last baseline in a flex line, and a block container's
//! intrinsic height with `row-gap` beside inline content.

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

fn boxed(dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str, text: &str) -> NodeId {
    let id = el(dom, parent, tag, class);
    let t = dom.create_text_node(text);
    dom.append_child(id, t).unwrap();
    id
}

/// CSS Box Alignment 3 §9.1: "for legacy reasons, if its
/// `baseline-source` is `auto` (the initial value) a block-level or
/// inline-level block container that is a scroll container always has a
/// last baseline set, whose baselines all correspond to its block-end
/// margin edge" — an `overflow: hidden` item's last baseline is its
/// bottom edge, not its content's last row.
#[test]
fn a_scroll_containers_last_baseline_is_its_block_end_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    boxed(&mut dom, f, "div", "a", "a");
    let b = boxed(&mut dom, f, "div", "", "b");
    lay_out(
        &mut dom,
        ".f { display: flex; align-items: last baseline } \
         .a { overflow: hidden; padding-bottom: 2 }",
        10,
        4,
    );
    assert_eq!(rect(&dom, b).y, 2);
}

/// CSS Box Alignment 3 §8 as rdom applies `row-gap` in block flow — the
/// gutters between block-level siblings, none around an inline run's
/// anonymous block: the intrinsic height of a block container counts a
/// gap only between its block-level children, as its layout does (1 +
/// 1 + 2 + 1 rows here).
#[test]
fn the_intrinsic_row_gap_skips_inline_children() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let col = el(&mut dom, root, "div", "col");
    let d = el(&mut dom, col, "div", "d");
    boxed(&mut dom, d, "p", "", "a");
    boxed(&mut dom, d, "span", "", "b");
    boxed(&mut dom, d, "p", "", "c");
    lay_out(
        &mut dom,
        ".col { display: flex; flex-direction: column; align-items: flex-start } \
         .d { row-gap: 2 }",
        10,
        10,
    );
    assert_eq!(rect(&dom, d).height, 5);
}
