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
            style(display, flow).is_atomic_inline(),
            atomic,
            "{display:?} {flow:?}"
        );
    }
}

/// C6G-ORDER-ALLOC: walking a box's children in paint order (CSS
/// Flexbox §5.4) allocates nothing unless an item's `order` is not 0 —
/// the walk runs per node per paint and hit-test, both ways.
#[test]
fn paint_order_allocates_only_for_reordered_items() {
    use crate::test_alloc::allocations_in;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let flex = dom.create_element("div");
    dom.set_attribute(flex, "class", "f").unwrap();
    dom.append_child(root, flex).unwrap();
    let mut items = Vec::new();
    for _ in 0..3 {
        let item = dom.create_element("div");
        dom.append_child(flex, item).unwrap();
        items.push(item);
    }
    let sheet = rdom_style::Stylesheet::new()
        .rule(
            ".f",
            rdom_style::TuiStyle::new()
                .display(crate::layout::Display::Block)
                .flow(crate::layout::Flow::Flex),
        )
        .unwrap()
        .rule(".o", rdom_style::TuiStyle::new().order(-1))
        .unwrap();
    dom.cascade(&sheet);
    for id in [flex, items[0]] {
        let mut seen = Vec::with_capacity(8);
        let n = allocations_in(|| {
            seen.extend(super::paint_order_children(&dom, id));
            seen.extend(super::paint_order_children(&dom, id).rev());
        });
        assert_eq!(n, 0, "{id:?}");
        if id == flex {
            assert_eq!(seen.len(), 6);
            assert_eq!(seen[..3], items[..]);
        }
    }
    dom.set_attribute(items[2], "class", "o").unwrap();
    dom.cascade(&sheet);
    let order: Vec<_> = super::paint_order_children(&dom, flex).collect();
    assert_eq!(order, [items[2], items[0], items[1]]);
    let back: Vec<_> = super::paint_order_children(&dom, flex).rev().collect();
    assert_eq!(back, [items[1], items[0], items[2]]);
}
