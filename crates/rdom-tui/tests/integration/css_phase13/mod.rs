//! CSS-COMPLETE Phase 13 — tables (CSS 2.1 §17, CSS Tables 3): each
//! sheet parsed strictly and cascaded through rdom-css, the markup parsed
//! by rdom-parser, the claim read from the painted cells, the laid-out
//! rects or the computed style. One submodule per concern; each test
//! cites the section that fixes the result.

use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, NodeId, PaintExt, TuiDom};

mod columns;
mod display;
mod gaps;
mod html;
mod props;
mod tfc;

/// A document holding `markup` under its root.
pub(crate) fn doc(markup: &str) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).expect("markup parses");
    dom
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

/// The first element with `id`.
pub(crate) fn by_id(dom: &TuiDom, id: &str) -> NodeId {
    dom.get_element_by_id(id)
        .unwrap_or_else(|| panic!("no #{id}"))
}
