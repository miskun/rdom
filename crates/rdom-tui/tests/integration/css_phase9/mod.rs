//! CSS-COMPLETE Phase 9 — inline text (CSS Text 3 / 4): each sheet
//! parsed strictly, cascaded, laid out and painted where the cells are
//! the claim. One submodule per item; each test cites the spec text that
//! fixes the expected result. The sheet helpers are Phase 5's.

#[allow(unused_imports)]
pub(crate) use super::css_phase5::{el, lay_out, paint, rect, rows, size};

mod breaking;
mod line_height;
mod tab_size;
mod text_align;
mod text_indent;
mod text_transform;
mod text_wrap;
mod white_space;

use rdom_tui::{NodeId, TuiDom};

/// A `.b` block holding the text `text`, styled `.b { decl }`, painted
/// in a `w` × `h` viewport: its rows, trailing blanks kept.
pub(crate) fn paint_text(decl: &str, text: &str, w: u16, h: u16) -> Vec<String> {
    let (mut dom, _, _) = text_block(text);
    let buf = paint(&mut dom, &format!(".b {{ {decl} }}"), w, h);
    rows(&buf, w, h)
}

/// A dom with a `.b` block holding one text node `text`: the dom, the
/// block and the text node.
pub(crate) fn text_block(text: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node(text);
    dom.append_child(b, t).unwrap();
    (dom, b, t)
}
