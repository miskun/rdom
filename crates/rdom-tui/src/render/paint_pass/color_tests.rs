//! Paint of the CSS Color 4 / 5 color forms (CSS-COMPLETE Phase 3):
//! cascade → layout → paint into a `Buffer`, one section per item.

use super::*;
use crate::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────

/// Cascade `css` onto `dom`, lay out and paint into a `w` × `h`
/// buffer.
fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let area = Rect::new(0, 0, w, h);
    dom.cascade(&parsed.stylesheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

/// An element `tag` with `text`, appended to `parent`.
fn element(dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str, text: &str) -> NodeId {
    let el = dom.create_element(tag);
    if !class.is_empty() {
        dom.set_attribute(el, "class", class).unwrap();
    }
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(el, t).unwrap();
    }
    dom.append_child(parent, el).unwrap();
    el
}

fn row(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .filter_map(|x| buf.cell(x, y))
        .filter(|c| !c.is_spacer())
        .map(|c| c.symbol().to_string())
        .collect()
}

// ── transparent: CSS Color 4 §6.3 ───────────────────────────────

/// CSS Color 4 §6.3: `transparent` is transparent black, so text in
/// that color is invisible — the cells show no glyph.
#[test]
fn transparent_text_paints_no_glyph() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "t", "hi");
    let buf = paint(&mut dom, ".t { color: transparent }", 6, 1);
    assert_eq!(row(&buf, 0).trim(), "");
}

/// A transparent background over the parent's shows the parent's.
#[test]
fn transparent_background_shows_the_parent_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = element(&mut dom, root, "div", "o", "");
    element(&mut dom, outer, "div", "i", "x");
    let buf = paint(
        &mut dom,
        ".o { background-color: blue } .i { background-color: transparent }",
        4,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Rgb(0, 0, 255));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "x");
}

/// Transparent text keeps its element's background.
#[test]
fn transparent_text_keeps_its_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = element(&mut dom, root, "p", "", "");
    let t = dom.create_text_node("ab ");
    dom.append_child(p, t).unwrap();
    element(&mut dom, p, "span", "s", "cd");
    let buf = paint(
        &mut dom,
        ".s { display: inline; color: transparent; background-color: red }",
        8,
        1,
    );
    assert_eq!(row(&buf, 0).trim_end(), "ab");
    assert_eq!(buf.cell(3, 0).unwrap().bg, Color::Rgb(255, 0, 0));
    assert_eq!(buf.cell(4, 0).unwrap().bg, Color::Rgb(255, 0, 0));
}

/// A transparent border draws no glyph but keeps its space.
#[test]
fn transparent_border_draws_nothing_but_takes_space() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "b", "x");
    let buf = paint(
        &mut dom,
        ".b { border-style: solid; border-color: transparent; width: 3; height: 3 }",
        3,
        3,
    );
    assert_eq!(row(&buf, 0).trim(), "");
    assert_eq!(row(&buf, 1), " x ");
    assert_eq!(row(&buf, 2).trim(), "");
}

/// Transparent text over other text leaves the text beneath visible.
#[test]
fn transparent_text_does_not_hide_the_backdrop() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "", "abc");
    element(&mut dom, root, "div", "over", "xyz");
    let buf = paint(
        &mut dom,
        ".over { position: absolute; top: 0; left: 0; color: transparent }",
        5,
        2,
    );
    assert_eq!(row(&buf, 0).trim_end(), "abc");
}

// ── Color alpha: CSS Color 4 §4.2, composited per cell ──────────

/// `rgb(… / 50%)`'s alpha as a fraction, and `c` composited over `d`.
fn over(c: (u8, u8, u8), d: (u8, u8, u8)) -> Color {
    let a = 128.0 / 255.0;
    let mix = |s: u8, b: u8| (a * f32::from(s) + (1.0 - a) * f32::from(b)).round() as u8;
    Color::Rgb(mix(c.0, d.0), mix(c.1, d.1), mix(c.2, d.2))
}

/// A translucent background blends with the background beneath it.
#[test]
fn translucent_background_blends_with_the_backdrop() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = element(&mut dom, root, "div", "o", "");
    element(&mut dom, outer, "div", "i", "x");
    let buf = paint(
        &mut dom,
        ".o { background-color: blue } .i { background-color: rgb(255 0 0 / 50%) }",
        4,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().bg, over((255, 0, 0), (0, 0, 255)));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "x");
    assert_eq!(buf.cell(3, 0).unwrap().bg, over((255, 0, 0), (0, 0, 255)));
}

/// A translucent background over text leaves the text, tinted toward
/// it (the group-opacity tint rule).
#[test]
fn translucent_background_keeps_the_text_beneath() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "t", "abc");
    element(&mut dom, root, "div", "over", "");
    let buf = paint(
        &mut dom,
        ".t { color: white; background-color: black } \
         .over { position: absolute; top: 0; left: 0; width: 5; height: 1; \
                 background-color: rgb(255 0 0 / 50%) }",
        5,
        2,
    );
    assert_eq!(row(&buf, 0).trim_end(), "abc");
    assert_eq!(buf.cell(0, 0).unwrap().bg, over((255, 0, 0), (0, 0, 0)));
    assert_eq!(
        buf.cell(0, 0).unwrap().fg,
        over((255, 0, 0), (255, 255, 255))
    );
}

/// A translucent foreground blends the glyph with the cell's
/// background.
#[test]
fn translucent_text_blends_with_the_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "t", "hi");
    let buf = paint(
        &mut dom,
        ".t { background-color: blue; color: rgb(255 0 0 / 50%) }",
        4,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(buf.cell(0, 0).unwrap().fg, over((255, 0, 0), (0, 0, 255)));
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Rgb(0, 0, 255));
}

/// Over another glyph, translucent text follows the glyph contest: at
/// alpha ≥ 0.5 it takes the cell, below it the backdrop's glyph stays.
#[test]
fn translucent_text_contests_the_backdrop_glyph() {
    let css = |alpha: &str| {
        format!(".over {{ position: absolute; top: 0; left: 0; color: rgb(255 0 0 / {alpha}) }}")
    };
    for (alpha, shown) in [("60%", "xyz"), ("40%", "abc")] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        element(&mut dom, root, "div", "", "abc");
        element(&mut dom, root, "div", "over", "xyz");
        let buf = paint(&mut dom, &css(alpha), 5, 2);
        assert_eq!(row(&buf, 0).trim_end(), shown, "alpha {alpha}");
    }
}

/// A translucent border blends its glyphs with the background.
#[test]
fn translucent_border_blends() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "b", "");
    let buf = paint(
        &mut dom,
        ".b { width: 3; height: 3; border-style: solid; background-color: black; \
              border-color: rgb(255 255 255 / 50%) }",
        3,
        3,
    );
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(0, 0).unwrap().fg, over((255, 255, 255), (0, 0, 0)));
}

/// Over the terminal's default background, a translucent color blends
/// with the canvas model of the document's color scheme.
#[test]
fn translucent_color_over_the_terminal_default_follows_the_scheme() {
    use rdom_style::color::ColorScheme;
    for (scheme, canvas) in [
        (ColorScheme::Dark, (0, 0, 0)),
        (ColorScheme::Light, (255, 255, 255)),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        element(&mut dom, root, "div", "t", "x");
        dom.set_color_scheme(scheme);
        let buf = paint(
            &mut dom,
            ".t { background-color: rgb(255 0 0 / 50%) }",
            2,
            1,
        );
        assert_eq!(
            buf.cell(0, 0).unwrap().bg,
            over((255, 0, 0), canvas),
            "{scheme:?}"
        );
    }
}

/// A translucent `::backdrop` dims what is beneath the modal dialog.
#[test]
fn translucent_backdrop_dims_the_page() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "page", "abc");
    let dialog = element(&mut dom, root, "dialog", "", "");
    dom.set_attribute(dialog, "open", "").unwrap();
    dom.set_attribute(dialog, "data-rdom-modal", "").unwrap();
    let buf = paint(
        &mut dom,
        ".page { color: white; background-color: blue } \
         dialog { display: none } \
         dialog::backdrop { background-color: rgb(0 0 0 / 50%) }",
        5,
        2,
    );
    assert_eq!(row(&buf, 0).trim_end(), "abc");
    assert_eq!(buf.cell(0, 0).unwrap().bg, over((0, 0, 0), (0, 0, 255)));
    assert_eq!(buf.cell(0, 0).unwrap().fg, over((0, 0, 0), (255, 255, 255)));
}
