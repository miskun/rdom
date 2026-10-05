//! C6G-CONTENTS-BOXTREE: the box-tree walks visit each node a bounded
//! number of times however deeply `display: contents` elements nest.

use std::cell::Cell;

use crate::{CascadeExt, TuiDom};

thread_local! {
    /// Child nodes the box-tree walks looked at.
    pub(super) static VISITS: Cell<usize> = const { Cell::new(0) };
}

/// `depth` nested `display: contents` divs around a block, in a block
/// container; the nodes `box_sequence` of the container visits.
fn visits(depth: usize) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    dom.append_child(root, container).unwrap();
    let mut parent = container;
    for _ in 0..depth {
        let div = dom.create_element("div");
        dom.set_attribute(div, "class", "c").unwrap();
        dom.append_child(parent, div).unwrap();
        parent = div;
    }
    let block = dom.create_element("div");
    dom.append_child(parent, block).unwrap();
    let sheet = rdom_style::Stylesheet::new()
        .rule(
            ".c",
            rdom_style::TuiStyle::new().display(crate::layout::Display::Contents),
        )
        .unwrap();
    dom.cascade(&sheet);
    VISITS.with(|c| c.set(0));
    let seq = super::box_sequence(&dom, container);
    assert_eq!(seq, [super::BoxItem::Node(block)]);
    VISITS.with(Cell::get)
}

/// CSS Display 3 §2.5: a box-less child holding a block box is replaced
/// by its children — decided in the same walk that collects them, so
/// each level is visited once (it was walked again per enclosing level:
/// quadratic in the depth).
#[test]
fn box_sequence_visits_each_node_once() {
    for depth in [4, 16] {
        let n = visits(depth);
        assert!(n <= depth + 1, "{depth} levels: {n} visits");
    }
}

/// CSS Display 3 §2.4: every inline-level box whose inner display is
/// not `flow` is atomic; `inline flow` is not.
#[test]
fn atomic_inlines_are_the_non_flow_inline_level_boxes() {
    use crate::layout::{Display, Flow};
    let style = |display, flow| {
        let mut c = crate::style::ComputedStyle::initial();
        c.display = display;
        c.flow = flow;
        c
    };
    for (display, flow, atomic) in [
        (Display::InlineBlock, Flow::Block, true),
        (Display::Inline, Flow::Flex, true),
        (Display::Inline, Flow::FlowRoot, true),
        (Display::Inline, Flow::Block, false),
        (Display::Block, Flow::Flex, false),
    ] {
        assert_eq!(
            super::is_atomic_inline(&style(display, flow)),
            atomic,
            "{display:?} {flow:?}"
        );
    }
}
