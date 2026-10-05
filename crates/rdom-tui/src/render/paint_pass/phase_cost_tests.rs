//! C8G-PAINT-PHASES — the cost of painting in Appendix E's phases: the
//! background phase is gathered by the walk that collects the stacking
//! context's layers, and the inline-content phase is the content
//! recursion that already ran, so a paint still looks at each node a
//! fixed number of times, and paints each block's box exactly once.

use super::stacking_walk::BOX_PAINTS;
use crate::prelude::*;
use crate::render::stacking::VISITS;

/// Paint `<body>` holding `n` paragraphs (each with a text child) nested
/// `depth` deep: the node visits of the paint walks and the boxes the
/// background phases painted.
fn paint_counts(n: usize, depth: usize) -> (usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut parent = dom.create_element("body");
    dom.append_child(root, parent).unwrap();
    for _ in 0..depth {
        let d = dom.create_element("div");
        dom.append_child(parent, d).unwrap();
        parent = d;
    }
    for _ in 0..n {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(parent, p).unwrap();
    }
    let sheet = rdom_css::from_css_strict("p { background-color: rgb(0, 0, 200) }").unwrap();
    let area = Rect::new(0, 0, 20, 200);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    VISITS.with(|c| c.set(0));
    BOX_PAINTS.with(|c| c.set(0));
    dom.paint_dom(&mut buf, area);
    (VISITS.with(|c| c.get()), BOX_PAINTS.with(|c| c.get()))
}

/// Each node is looked at twice — once collecting the context's layers
/// and background-phase boxes, once in the content recursion — and each
/// block box (`<body>`, the `<div>`s, the `<p>`s) is painted once: no
/// walk per phase, no box painted twice.
#[test]
fn the_phases_add_no_walk_and_paint_each_box_once() {
    for (n, depth) in [(10, 0), (40, 0), (5, 20)] {
        let (visits, boxes) = paint_counts(n, depth);
        // The nodes: `<body>`, the `<div>`s, the `<p>`s and their texts.
        let nodes = 1 + depth + 2 * n;
        assert_eq!(visits, 2 * nodes, "n {n}, depth {depth}");
        assert_eq!(boxes, 1 + depth + n, "n {n}, depth {depth}");
    }
}
