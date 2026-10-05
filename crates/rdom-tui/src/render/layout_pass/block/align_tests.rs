//! C6G-BLOCK-ALIGN: `align-content` on a block container (CSS Box
//! Alignment 3 §5.1) moves the content it laid out; it does not lay it
//! out again, so nested aligned containers cost one layout each.

use std::cell::Cell;

use crate::render::Rect;
use crate::render::layout_pass::LAYOUTS;
use crate::{CascadeExt, LayoutExt, TuiDom};

/// `depth` nested containers with `align-content: center`, each two rows
/// taller than the next (the innermost two rows taller than its one-row
/// leaf), in a block-flow wrapper; the elements one layout pass laid out.
fn layouts(depth: usize) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrapper = dom.create_element("div");
    dom.append_child(root, wrapper).unwrap();
    let mut parent = wrapper;
    let mut css = String::new();
    for k in 0..depth {
        let div = dom.create_element("div");
        dom.set_attribute(div, "class", &format!("a{k}")).unwrap();
        css += &format!(
            ".a{k} {{ height: {}; align-content: center }} ",
            2 * (depth - k) + 1
        );
        dom.append_child(parent, div).unwrap();
        parent = div;
    }
    let leaf = dom.create_element("div");
    dom.append_child(parent, leaf).unwrap();
    let text = dom.create_text_node("x");
    dom.append_child(leaf, text).unwrap();
    let sheet = rdom_css::from_css_strict(&css).expect("sheet parses");
    dom.cascade(&sheet);
    LAYOUTS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 20, 40));
    LAYOUTS.with(Cell::get)
}

/// Each element is laid out once (the leaf, its `depth` aligned
/// ancestors and the wrapper): re-laying the content at its offset
/// doubled the layouts under each aligned ancestor — 2^depth for the
/// leaf.
#[test]
fn nested_aligned_containers_lay_out_each_box_once() {
    for depth in [3, 8] {
        let n = layouts(depth);
        assert_eq!(n, depth + 2, "{depth} levels");
    }
}
