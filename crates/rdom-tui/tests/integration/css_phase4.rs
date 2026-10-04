//! CSS-COMPLETE Phase 4 — backgrounds and borders end to end: a sheet
//! parsed by `rdom_css::from_css_strict` (so no declaration may be
//! dropped), cascaded, laid out and painted, asserting painted cells.
//! One section per item; each test cites the spec text that fixes the
//! expected cells.

use rdom_tui::render::{Buffer, Cell, Rect};
use rdom_tui::{CascadeExt, Color, LayoutExt, NodeId, PaintExt, TuiDom};

/// A `div` with `class` (and `text`, if any) appended to `parent`.
fn el(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let id = dom.create_element("div");
    dom.set_attribute(id, "class", class).unwrap();
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (no warning allowed), lay out and paint into a
/// `w` × `h` buffer.
fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
    let area = Rect::new(0, 0, w, h);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

fn cell(buf: &Buffer, x: u16, y: u16) -> &Cell {
    buf.cell(x, y).expect("in the buffer")
}

// ── C4-BACKGROUND ──────────────────────────────────────────────────

/// CSS Backgrounds 3 §3.10: the full shorthand — an image layer with a
/// position, size, repeat and attachment, then the color — parses, and
/// its color paints the box. The image layer draws nothing.
#[test]
fn background_shorthand_with_an_image_layer_paints_its_color() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    let buf = paint(
        &mut dom,
        ".b { width: 3; height: 1; \
              background: url(tile.png) center / cover no-repeat fixed rgb(10, 20, 30) }",
        4,
        1,
    );
    for x in 0..3 {
        assert_eq!(cell(&buf, x, 0).bg, Color::Rgb(10, 20, 30), "x={x}");
    }
    assert_eq!(cell(&buf, 3, 0).bg, Color::Reset);
}

/// §3.10: two layers, the color on the final one.
#[test]
fn background_layers_paint_the_final_layers_color() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    let buf = paint(
        &mut dom,
        ".b { width: 2; height: 1; \
              background: linear-gradient(red, blue), url(a.png) repeat-x rgb(1, 2, 3) }",
        2,
        1,
    );
    assert_eq!(cell(&buf, 0, 0).bg, Color::Rgb(1, 2, 3));
}
