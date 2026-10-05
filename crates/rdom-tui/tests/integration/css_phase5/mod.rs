//! CSS-COMPLETE Phase 5 — box model and sizing: each sheet parsed
//! strictly, cascaded and laid out (and painted where the cells are
//! the claim). One submodule per item; each test cites the spec text
//! that fixes the expected geometry.

use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, LayoutRect, NodeId, PaintExt, TuiDom, TuiNodeExt};

mod box_sizing;
mod intrinsic;

/// A `tag` element with `class`, appended to `parent`.
fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let id = dom.create_element(tag);
    if !class.is_empty() {
        dom.set_attribute(id, "class", class).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (strict: no warnings) and lay the tree out in `w` × `h`.
fn lay_out(dom: &mut TuiDom, css: &str, w: u16, h: u16) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, w, h));
}

/// Cascade, lay out and paint `css` in `w` × `h`.
fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    lay_out(dom, css, w, h);
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

/// `id`'s border-box rect.
fn rect(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).layout_rect().expect("laid out")
}

/// `(width, height)` of `id`'s border box.
fn size(dom: &TuiDom, id: NodeId) -> (u16, u16) {
    let r = rect(dom, id);
    (r.width, r.height)
}

/// The glyphs of a `w` × `h` region, row by row.
fn rows(buf: &Buffer, w: u16, h: u16) -> Vec<String> {
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf.cell(x, y).expect("in the buffer").symbol().to_string())
                .collect()
        })
        .collect()
}
