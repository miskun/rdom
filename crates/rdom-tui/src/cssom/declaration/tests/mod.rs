//! Unit tests for the CSSOM declaration wrappers, grouped by
//! concern: `read` (getters, `length` / `item`, generated aliases),
//! `write` (the setter family and its error channel), and
//! `css_text` (serialization shape + round-trips).

use crate::TuiDom;

mod css_text;
mod read;
mod write;

fn dom_with(tag: &str) -> (TuiDom, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element(tag);
    dom.append_child(root, el).unwrap();
    (dom, el)
}
