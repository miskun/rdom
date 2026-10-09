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
    dom.add_to_top_layer(dialog, rdom_core::TopLayerKind::ModalDialog)
        .unwrap();
    // The dialog is rendered (a backdrop exists only for a rendered
    // top-layer element) but empty and away from cell (0, 0).
    let buf = paint(
        &mut dom,
        ".page { color: white; background-color: blue } \
         dialog { border: none; padding: 0; width: 1; height: 1; inset: auto 0 0 auto } \
         dialog::backdrop { background-color: rgb(0 0 0 / 50%) }",
        5,
        2,
    );
    assert_eq!(row(&buf, 0).trim_end(), "abc");
    assert_eq!(buf.cell(0, 0).unwrap().bg, over((0, 0, 0), (0, 0, 255)));
    assert_eq!(buf.cell(0, 0).unwrap().fg, over((0, 0, 0), (255, 255, 255)));
}

/// ACID-FIX-13 (found by acid tile 15c). CSS 2.1 §14.1: `color` is the
/// foreground of an element's own text; `::backdrop` has none, and it
/// inherits its originating dialog's (CSS Position 4 §4). A dialog's
/// `color` must not recolor the page under its backdrop: the backdrop's
/// translucent background tints the page's glyphs (C3-ALPHA) and nothing
/// else does. The backdrop's `color` set every glyph beneath it.
#[test]
fn a_dialogs_color_does_not_recolor_the_page_under_its_backdrop() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "page", "abc");
    let dialog = element(&mut dom, root, "dialog", "", "");
    dom.set_attribute(dialog, "open", "").unwrap();
    dom.add_to_top_layer(dialog, rdom_core::TopLayerKind::ModalDialog)
        .unwrap();
    let buf = paint(
        &mut dom,
        ".page { color: white; background-color: blue } \
         dialog { color: red; border: none; padding: 0; width: 1; height: 1; inset: auto 0 0 auto } \
         dialog::backdrop { background-color: rgb(0 0 0 / 50%) }",
        5,
        2,
    );
    assert_eq!(buf.cell(0, 0).unwrap().fg, over((0, 0, 0), (255, 255, 255)));
}

// ── One composite per background (C3G-PSEUDO-TINT) ──────────────

/// The red-at-half-alpha background over black: what every cell of the
/// box — under its text too — must show, composited once.
fn half_red_over_black() -> Color {
    over((255, 0, 0), (0, 0, 0))
}

/// CSS Color 4 §4.2 / Backgrounds 3 §3.10: a box's background paints
/// once, under its content. A positioned pseudo-element's translucent
/// background is composited over its whole box, then its text is
/// written in a glyph style — so the cells under the text show the same
/// color as the rest of the box (they used to composite the background
/// a second time: (191, 0, 0) under the text, (128, 0, 0) beside it).
#[test]
fn positioned_pseudo_translucent_background_composites_once_under_its_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "h", "");
    let buf = paint(
        &mut dom,
        ".h { position: relative; width: 6; height: 1; background-color: black } \
         .h::after { position: absolute; top: 0; left: 0; right: 2; \
                     content: \"hi\"; background-color: rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        half_red_over_black(),
        "under text"
    );
    assert_eq!(
        buf.cell(3, 0).unwrap().bg,
        half_red_over_black(),
        "beside it"
    );
}

/// The same for every `::before` / `::after` laid out as a box of its
/// own — here a float (C10-PSEUDO-UNIFY): its box paints its background,
/// and its text is written over it in a glyph style.
#[test]
fn floated_pseudo_translucent_background_composites_once_under_its_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "h", "");
    let buf = paint(
        &mut dom,
        ".h { width: 6; height: 1; background-color: black } \
         .h::before { float: left; width: 4; content: \"hi\"; \
                      background-color: rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        half_red_over_black(),
        "under text"
    );
    assert_eq!(
        buf.cell(3, 0).unwrap().bg,
        half_red_over_black(),
        "beside it"
    );
}

/// A static `::before` has no box fill of its own: its background
/// paints only with its text, once.
#[test]
fn static_pseudo_translucent_background_composites_once() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    element(&mut dom, root, "div", "h", "z");
    let buf = paint(
        &mut dom,
        ".h { background-color: black } \
         .h::before { content: \"ab\"; background-color: rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(row(&buf, 0).trim_end(), "abz");
    assert_eq!(buf.cell(0, 0).unwrap().bg, half_red_over_black());
    assert_eq!(buf.cell(2, 0).unwrap().bg, Color::Rgb(0, 0, 0), "host text");
}

/// An inline element's background paints with its fragments (it has no
/// box fill): composited once over the block's.
#[test]
fn inline_translucent_background_composites_once() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let block = element(&mut dom, root, "div", "b", "");
    let text = dom.create_text_node("a");
    dom.append_child(block, text).unwrap();
    element(&mut dom, block, "span", "s", "xy");
    let buf = paint(
        &mut dom,
        ".b { background-color: black } \
         .s { display: inline; background-color: rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(row(&buf, 0).trim_end(), "axy");
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        Color::Rgb(0, 0, 0),
        "block text"
    );
    assert_eq!(buf.cell(1, 0).unwrap().bg, half_red_over_black());
    assert_eq!(buf.cell(2, 0).unwrap().bg, half_red_over_black());
    assert_eq!(buf.cell(3, 0).unwrap().bg, Color::Rgb(0, 0, 0));
}

/// A block's own text over its translucent background: the fill
/// composites once and the text adds none.
#[test]
fn block_translucent_background_composites_once_under_its_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = element(&mut dom, root, "div", "o", "");
    element(&mut dom, outer, "div", "i", "xy");
    let buf = paint(
        &mut dom,
        ".o { background-color: black } .i { background-color: rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        half_red_over_black(),
        "under text"
    );
    assert_eq!(
        buf.cell(4, 0).unwrap().bg,
        half_red_over_black(),
        "beside it"
    );
}

/// A tree row's translucent highlight composites once over the row,
/// label cells included.
#[test]
fn tree_row_translucent_highlight_composites_once_under_its_label() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let tree = element(&mut dom, root, "ul", "t", "");
    dom.set_attribute(tree, "role", "tree").unwrap();
    let item = element(&mut dom, tree, "li", "", "ab");
    dom.set_attribute(item, "role", "treeitem").unwrap();
    dom.set_attribute(item, "aria-selected", "true").unwrap();
    let buf = paint(
        &mut dom,
        ".t { background-color: black } \
         [role=treeitem][aria-selected=true] { background-color: rgb(255 0 0 / 50%) }",
        8,
        1,
    );
    let label = (0..8)
        .find(|&x| buf.cell(x, 0).unwrap().symbol() == "a")
        .expect("label painted");
    assert_eq!(
        buf.cell(label, 0).unwrap().bg,
        half_red_over_black(),
        "label"
    );
    assert_eq!(buf.cell(7, 0).unwrap().bg, half_red_over_black(), "row end");
}

// ── One canvas model (C3G-SCHEME-CONSISTENCY) ────────────────────

/// A translucent tree guide (the treeitem's `border-color`) is drawn by
/// the border joiner; it composites through the buffer like a
/// translucent border, so over the terminal's default background it
/// blends with the canvas of the document's scheme — white when light
/// (the joiner wrote it into the cell, which blended against black in
/// every scheme).
#[test]
fn translucent_tree_guide_blends_with_the_scheme_canvas() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let tree = element(&mut dom, root, "ul", "", "");
    dom.set_attribute(tree, "role", "tree").unwrap();
    let branch = element(&mut dom, tree, "li", "", "A");
    dom.set_attribute(branch, "role", "treeitem").unwrap();
    dom.set_attribute(branch, "aria-expanded", "true").unwrap();
    let group = element(&mut dom, branch, "ul", "", "");
    dom.set_attribute(group, "role", "group").unwrap();
    let leaf = element(&mut dom, group, "li", "", "b");
    dom.set_attribute(leaf, "role", "treeitem").unwrap();
    dom.set_color_scheme(rdom_style::color::ColorScheme::Light);
    // With the UA sheet: it indents the group.
    let sheet =
        rdom_css::from_css_strict("[role=treeitem] { border-color: rgb(255 0 0 / 50%) }").unwrap();
    let area = Rect::new(0, 0, 10, 2);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    let connector = buf.cell(0, 1).unwrap();
    assert_eq!(connector.symbol(), "└");
    assert_eq!(connector.fg, over((255, 0, 0), (255, 255, 255)));
}
