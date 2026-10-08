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

/// The box-tree visits a layout of `rows` sibling `<div>row</div>` blocks in
/// one plain block container makes (`sheet` cascaded first).
fn layout_visits(rows: usize, sheet: &str) -> usize {
    use crate::prelude::*;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("div");
    dom.set_attribute(list, "class", "list").unwrap();
    dom.append_child(root, list).unwrap();
    for _ in 0..rows {
        let row = dom.create_element("div");
        let t = dom.create_text_node("row");
        dom.append_child(row, t).unwrap();
        dom.append_child(list, row).unwrap();
    }
    dom.cascade(&rdom_css::from_css_strict(sheet).unwrap());
    VISITS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    VISITS.with(std::cell::Cell::get)
}

/// C10G-MARKER-COST — a list item's marker rides the first line of the
/// block holding it (CSS Lists 3 §3.5), so placing markers climbs from a
/// packed line toward list-item ancestors. A document with no list item
/// must not pay for that: 2000 sibling rows in a plain `<div>` cost a
/// bounded number of box-tree visits per row (each row's pack climbed to
/// its parent and rebuilt the parent's whole box sequence — quadratic).
#[test]
fn sibling_rows_lay_out_in_linear_box_tree_visits() {
    for sheet in [
        "",
        // Every row a list item: each marker rides its own row's line.
        ".list > div { display: list-item }",
        // A list item elsewhere: the climb runs for every row.
        ".list > div:first-child { display: list-item }",
    ] {
        let (small, large) = (layout_visits(200, sheet), layout_visits(2000, sheet));
        assert!(
            large <= 16 * 2000,
            "{sheet:?}: {large} visits for 2000 rows ({small} for 200)"
        );
    }
}

/// `find_in_sequence` finds what a search of `box_sequence` finds, from
/// either end, through box-less children that hold a block box (their
/// items in place, between their inline-level pseudo-elements) and those
/// that do not (one item), skipping a closed `<details>`' hidden text.
#[test]
fn find_in_sequence_agrees_with_box_sequence() {
    use crate::prelude::*;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    // host: span.c("a"), div.c[ p("b"), span("c") ], "  "
    let add = |dom: &mut TuiDom, parent, tag: &str, class: &str, text: Option<&str>| {
        let el = dom.create_element(tag);
        if !class.is_empty() {
            dom.set_attribute(el, "class", class).unwrap();
        }
        if let Some(text) = text {
            let t = dom.create_text_node(text);
            dom.append_child(el, t).unwrap();
        }
        dom.append_child(parent, el).unwrap();
        el
    };
    add(&mut dom, host, "span", "c", Some("a"));
    let boxless = add(&mut dom, host, "div", "c", None);
    add(&mut dom, boxless, "p", "", Some("b"));
    add(&mut dom, boxless, "span", "", Some("c"));
    let ws = dom.create_text_node("  ");
    dom.append_child(host, ws).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".c { display: contents } .c::before { content: \"[\" } .c::after { content: \"]\" }\
         div::after { content: \"z\"; display: block }",
    )
    .unwrap();
    dom.cascade(&sheet);
    let seq = super::box_sequence(&dom, host);
    assert!(seq.len() >= 5, "{seq:?}");
    let preds: [&dyn Fn(super::BoxItem) -> bool; 3] = [&|_| true, &|i| i.node().is_none(), &|i| {
        i.node()
            .is_some_and(|n| dom.node(n).tag_name() == Some("p"))
    }];
    for pred in preds {
        for from_end in [false, true] {
            let want = if from_end {
                seq.iter().rev().copied().find(|&i| pred(i))
            } else {
                seq.iter().copied().find(|&i| pred(i))
            };
            let got = super::find_in_sequence(&dom, host, from_end, &mut |i| pred(i));
            assert_eq!(got, want, "from_end {from_end}");
        }
    }
}

// ── The `::details-content` box (C10G-DETAILS-CONTENT-BOX) ──────────

/// HTML §15.5.20: the first slot takes the first `<summary>`, the second
/// everything else — so the `<details>`'s box-tree children are the
/// summary and the slot's box, in that order wherever the summary is,
/// and the box's are the other children in tree order; the climb from
/// slotted content goes through the box to the `<details>`.
#[test]
fn a_details_box_tree_is_its_summary_and_its_slots_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    dom.append_child(root, d).unwrap();
    let loose = dom.create_text_node("loose");
    dom.append_child(d, loose).unwrap();
    let summary = dom.create_element("summary");
    dom.append_child(d, summary).unwrap();
    let p = dom.create_element("p");
    dom.append_child(d, p).unwrap();
    let second = dom.create_element("summary");
    dom.append_child(d, second).unwrap();
    dom.cascade(&rdom_style::Stylesheet::new());
    let slot = super::slot::content_box(&dom, d).expect("a `<details>` has a slot box");
    assert_eq!(
        super::children(&dom, d).collect::<Vec<_>>(),
        [summary, slot]
    );
    assert_eq!(
        super::children(&dom, d).rev().collect::<Vec<_>>(),
        [slot, summary]
    );
    let content = [loose, p, second];
    assert_eq!(super::children(&dom, slot).collect::<Vec<_>>(), content);
    assert_eq!(
        super::children(&dom, slot).rev().collect::<Vec<_>>(),
        [second, p, loose]
    );
    assert_eq!(super::box_parent(&dom, p), Some(slot));
    assert_eq!(super::box_parent(&dom, slot), Some(d));
    assert_eq!(super::box_parent(&dom, summary), Some(d));
    assert_eq!(super::slot::host_of(&dom, slot), Some(d));
    assert_eq!(
        dom.node(slot).parent_node().map(|n| n.id()),
        None,
        "not in the DOM"
    );
}
