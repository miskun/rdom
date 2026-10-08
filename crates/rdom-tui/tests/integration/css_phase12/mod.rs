//! CSS-COMPLETE Phase 12 part 3 — user interface (CSS UI 4): each sheet
//! parsed strictly and cascaded through rdom-css, the claim read from the
//! painted cells or the computed style. One submodule per item; each test
//! cites the CSS UI 4 section that fixes the result.

use rdom_tui::render::{Buffer, Cell, Rect};
use rdom_tui::{CascadeExt, LayoutExt, NodeId, PaintExt, TuiDom};

mod outline;

/// A `div` with `class` (and `text`, if any) appended to `parent`.
pub(crate) fn el(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let id = dom.create_element("div");
    dom.set_attribute(id, "class", class).unwrap();
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (no warning allowed), lay out and paint into a `w` ×
/// `h` buffer.
pub(crate) fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
    let area = Rect::new(0, 0, w, h);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

/// The painted rows, trailing blanks trimmed.
pub(crate) fn rows(buf: &Buffer) -> Vec<String> {
    crate::common::buffer_to_snapshot(buf)
        .lines()
        .map(str::to_string)
        .collect()
}

pub(crate) fn cell(buf: &Buffer, x: u16, y: u16) -> &Cell {
    buf.cell(x, y).expect("in the buffer")
}
