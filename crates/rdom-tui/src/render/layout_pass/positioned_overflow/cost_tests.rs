//! C8-ABSPOS-OVERFLOW: what counting positioned boxes in their scroll
//! containers' overflow costs — a measurement per placed box, and a
//! second run of phases 1–2 only when their reach changed.

use crate::render::Rect;
use crate::render::layout_pass::{LAYOUTS, ROUNDS};
use crate::{CascadeExt, LayoutExt, TuiDom};

use super::MEASURED;

/// Three 6 × 2 scroll containers, each holding a 1-row child and — when
/// `with_abs` — an absolutely positioned box past its content.
fn ports(with_abs: bool) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    for _ in 0..3 {
        let port = dom.create_element("div");
        dom.set_attribute(port, "class", "port").unwrap();
        dom.append_child(root, port).unwrap();
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "row").unwrap();
        dom.append_child(port, row).unwrap();
        if with_abs {
            let abs = dom.create_element("div");
            dom.set_attribute(abs, "class", "abs").unwrap();
            dom.append_child(port, abs).unwrap();
        }
    }
    let sheet = rdom_css::from_css_strict(
        ".port { width: 6; height: 2; overflow: hidden; position: relative } \
         .row { height: 1 } \
         .abs { position: absolute; top: 5; left: 8; width: 2; height: 1 }",
    )
    .expect("sheet parses");
    dom.cascade(&sheet);
    dom
}

/// `layout_dom` once: `(layout_node calls, positioned boxes measured)`.
fn counted(dom: &mut TuiDom) -> (usize, usize) {
    LAYOUTS.with(|c| c.set(0));
    MEASURED.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 20, 10));
    (LAYOUTS.with(|c| c.get()), MEASURED.with(|c| c.get()))
}

/// No positioned box: one run, nothing measured. Three boxes appearing:
/// phases 1–2 run twice (the reach changed), each box measured once a
/// run. Laid out again unchanged: one run — the in-flow layout plus one
/// placement per box — and one measurement per box.
#[test]
fn positioned_boxes_cost_one_measurement_and_a_rerun_only_on_change() {
    let mut plain = ports(false);
    let (base, measured) = counted(&mut plain);
    assert_eq!(measured, 0);
    assert_eq!(counted(&mut plain), (base, 0), "a settled document");

    let mut dom = ports(true);
    assert_eq!(counted(&mut dom), (2 * (base + 3), 6), "the reach appeared");
    assert_eq!(counted(&mut dom), (base + 3, 3), "the reach is unchanged");
}

/// C8G-ABSPOS-EXTENT (architect N1): a reach whose own scrollbar changes
/// it converges inside one `layout_dom`. A positioned 20 × 10 scroller
/// holding a `left: 0; right: 0; height: 30` box: run 2 sees the 30 rows
/// and adds the vertical bar, so the box is 19 wide — its reach changed
/// again. A third run measures it; the frame painted after this
/// `layout_dom` has no horizontal overflow (a 20-wide reach in a 19-wide
/// scrollport would scroll one column) — and the next layout runs once.
#[test]
fn a_reach_its_own_scrollbar_changes_converges_in_one_layout() {
    use crate::accessors::TuiAccessors;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = dom.create_element("div");
    dom.set_attribute(port, "class", "port").unwrap();
    dom.append_child(root, port).unwrap();
    let abs = dom.create_element("div");
    dom.set_attribute(abs, "class", "abs").unwrap();
    dom.append_child(port, abs).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".port { position: relative; overflow: auto; width: 20; height: 10 } \
         .abs { position: absolute; left: 0; right: 0; height: 30 }",
    )
    .expect("sheet parses");
    dom.cascade(&sheet);
    ROUNDS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 30, 12));
    assert_eq!(dom.node(port).scroll_range().unwrap().x(), 0..=0);
    assert_eq!(dom.node(port).scroll_width(), Some(19));
    assert_eq!(ROUNDS.with(|c| c.get()), 3, "stale, the bar, converged");
    ROUNDS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 30, 12));
    assert_eq!(ROUNDS.with(|c| c.get()), 1, "settled");
}
