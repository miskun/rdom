//! C8G-IDLE-COST — intrinsic measurement pays for floats only where one
//! is: a block container's inline sizes partition its children into
//! float runs only when it holds a float (CSS 2.1 §9.5,
//! `float::measure::block_width`).

use crate::prelude::*;
use crate::render::layout_pass::block::FLOW_RUNS;

/// A row flex container of `n` block items, each holding two paragraphs
/// (one with a float when `float`), laid out: the float-run partitions
/// the pass built.
fn flow_runs(n: usize, float: bool) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div");
    dom.set_attribute(row, "class", "row").unwrap();
    dom.append_child(root, row).unwrap();
    for _ in 0..n {
        let item = dom.create_element("div");
        dom.append_child(row, item).unwrap();
        for k in 0..2 {
            let p = dom.create_element("p");
            if float && k == 0 {
                dom.set_attribute(p, "class", "f").unwrap();
            }
            let t = dom.create_text_node("x");
            dom.append_child(p, t).unwrap();
            dom.append_child(item, p).unwrap();
        }
    }
    let sheet =
        rdom_css::from_css_strict(".row { display: flex } .f { float: left; width: 1 }").unwrap();
    dom.cascade(&sheet);
    FLOW_RUNS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    FLOW_RUNS.with(|c| c.get())
}

/// Without a float nothing is partitioned for it (each item's inline
/// sizes were, every measurement); with one, its container is.
#[test]
fn block_inline_sizes_partition_floats_only_when_one_floats() {
    assert_eq!(flow_runs(8, false), 0);
    assert!(flow_runs(8, true) > 0);
}
