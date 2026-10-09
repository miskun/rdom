//! CSS-COMPLETE Phase 15 part 1 — transforms, filters, compositing and
//! clipping (CSS Transforms 1 / 2, Filter Effects 1 / 2, Compositing and
//! Blending 1, CSS Masking 1): each sheet parsed strictly through rdom-css,
//! the markup by rdom-parser, the claim read from the computed style, the
//! laid-out rects, the painted cells or a hit test. One submodule per
//! concern; each test cites the section that fixes the result.

use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, Color, LayoutExt, NodeId, PaintExt, TuiDom, TuiNodeExt, Viewport};

mod blend;
mod filter;
mod translate;
mod translate_motion;

/// A document holding `markup` under its root.
pub(crate) fn doc(markup: &str) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).expect("markup parses");
    dom
}

/// The sheet `css`, parsed with no warning allowed.
pub(crate) fn sheet(css: &str) -> rdom_tui::Stylesheet {
    rdom_css::from_css_strict(css).expect("sheet parses without warnings")
}

/// Cascade `css` (no warning allowed) in a `w` × `h` viewport and lay out.
pub(crate) fn styled(dom: &mut TuiDom, css: &str, w: u16, h: u16) {
    let sheet = sheet(css);
    dom.set_viewport(Viewport::new(w, h));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, w, h));
}

/// [`styled`], then paint into a `w` × `h` buffer.
pub(crate) fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    styled(dom, css, w, h);
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

/// The first element with `id`.
pub(crate) fn by_id(dom: &TuiDom, id: &str) -> NodeId {
    dom.get_element_by_id(id)
        .unwrap_or_else(|| panic!("no #{id}"))
}

/// `id`'s border box: `(x, y, width, height)`.
pub(crate) fn rect(dom: &TuiDom, id: &str) -> (i32, i32, u16, u16) {
    let r = dom.node(by_id(dom, id)).layout_rect().expect("laid out");
    (r.x, r.y, r.width, r.height)
}

/// Row `y` of `buf` as text.
pub(crate) fn row(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
        .collect()
}

pub(crate) const RED: Color = Color::Rgb(255, 0, 0);
pub(crate) const BLUE: Color = Color::Rgb(0, 0, 255);
pub(crate) const GREEN: Color = Color::Rgb(0, 128, 0);
