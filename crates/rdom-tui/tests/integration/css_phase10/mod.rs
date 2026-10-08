//! CSS-COMPLETE Phase 10 — lists, counters, generated content and
//! pseudo-elements: each sheet parsed strictly, cascaded, laid out and
//! painted where the cells are the claim. One submodule per item; each
//! test cites the spec text that fixes the expected result. The sheet
//! helpers are Phase 5's.

#[allow(unused_imports)]
pub(crate) use super::css_phase5::{el, lay_out, paint, rect, rows, size};

mod content;
mod counter_style;
mod counters;
mod details_content;
mod first;
mod highlight;
mod legacy_colon;
mod list_item;
mod pseudo_chains;
mod pseudo_unify;
mod quotes;

use rdom_tui::{NodeId, TuiDom};

/// A `tag.class` element holding the text `text`, appended to `parent`.
pub(crate) fn text_el(
    dom: &mut TuiDom,
    parent: NodeId,
    tag: &str,
    class: &str,
    text: &str,
) -> NodeId {
    let id = el(dom, parent, tag, class);
    let t = dom.create_text_node(text);
    dom.append_child(id, t).unwrap();
    id
}

/// Paint `css` over the tree `build` makes under the root, in a `w` ×
/// `h` viewport: its rows, trailing blanks kept.
pub(crate) fn paint_tree(
    css: &str,
    w: u16,
    h: u16,
    build: impl FnOnce(&mut TuiDom, NodeId),
) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    build(&mut dom, root);
    let buf = paint(&mut dom, css, w, h);
    rows(&buf, w, h)
}
