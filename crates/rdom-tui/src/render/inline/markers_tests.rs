//! C10G-MARKER-COST — placing list markers (CSS Lists 3 §3.5) costs
//! nothing in a document with no list item.

use std::cell::Cell;

use crate::prelude::{CascadeExt, LayoutExt, Rect, TuiDom};

thread_local! {
    /// Steps `line_markers` climbed toward a list-item ancestor.
    pub(super) static CLIMB_STEPS: Cell<usize> = const { Cell::new(0) };
}

/// Climb steps a layout of `<div><div><p>text</p></div></div>` takes under
/// `sheet`.
fn climb_steps(sheet: &str) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut parent = root;
    for tag in ["div", "div", "p"] {
        let el = dom.create_element(tag);
        dom.append_child(parent, el).unwrap();
        parent = el;
    }
    let t = dom.create_text_node("text");
    dom.append_child(parent, t).unwrap();
    dom.cascade(&rdom_css::from_css_strict(sheet).unwrap());
    CLIMB_STEPS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    CLIMB_STEPS.with(Cell::get)
}

/// A marker rides a descendant's first line only under a list item
/// (§3.5): with none in the document the climb never starts; with one —
/// the outer `div` — it does, and the marker is placed.
#[test]
fn the_marker_climb_runs_only_in_a_document_with_list_items() {
    assert_eq!(climb_steps(""), 0);
    assert!(climb_steps("div:first-child { display: list-item }") > 0);
}
