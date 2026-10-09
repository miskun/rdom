//! ACID-FIX-3 (found by acid tile 9a) — a glyph painted over another is
//! drawn in its own style.
//!
//! CSS 2.1 Appendix E paints a stacking context's negative `z-index`
//! descendants before its in-flow text, and a terminal cell shows one
//! glyph — the one painted last. That glyph is the text's, so it shows in
//! the text's `color` (CSS Color 4 §3) — the initial `CanvasText`, the
//! terminal's default foreground, included — with the text's weight, slant
//! and decorations, never those of the glyph it replaced. The cell's
//! background is the boxes' and stays (CSS 2.1 Appendix E steps 3–7).

use rdom_tui::prelude::*;
use rdom_tui::render::Buffer;

use crate::common::render;

/// `.host` (`position: relative`, text `HOST`) over its own `::after`
/// (`z-index: -1`, `____` in `pseudo_css`, a navy background).
fn host_over_pseudo(host_css: &str, pseudo_css: &str) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("p");
    dom.set_attribute(host, "class", "host").unwrap();
    dom.append_child(root, host).unwrap();
    let t = dom.create_text_node("HOST");
    dom.append_child(host, t).unwrap();
    let sheet = rdom_css::from_css(&format!(
        ".host {{ position: relative; {host_css} }}
         .host::after {{ content: \"____\"; position: absolute; left: 0; top: 0;
           z-index: -1; background-color: rgb(0, 0, 128); {pseudo_css} }}"
    ));
    render(&mut dom, &sheet, Rect::new(0, 0, 8, 1))
}

#[test]
fn default_colored_text_over_a_colored_glyph_shows_the_default_color() {
    let buf = host_over_pseudo("", "color: rgb(192, 0, 0)");
    for x in 0..4 {
        let cell = buf.cell(x, 0).unwrap();
        assert_eq!(cell.symbol(), &"HOST"[x as usize..=x as usize]);
        assert_eq!(cell.fg, Color::Reset, "cell {x}: the text's CanvasText");
        assert_eq!(
            cell.bg,
            Color::Rgb(0, 0, 128),
            "cell {x}: the pseudo's background stays"
        );
    }
}

#[test]
fn plain_text_over_a_bold_underlined_glyph_is_plain() {
    let buf = host_over_pseudo(
        "color: rgb(0, 160, 0)",
        "font-weight: bold; font-style: italic; text-decoration: underline wavy rgb(0, 0, 255)",
    );
    let cell = buf.cell(1, 0).unwrap();
    assert_eq!(cell.symbol(), "O");
    assert_eq!(cell.fg, Color::Rgb(0, 160, 0));
    assert_eq!(
        cell.modifier,
        Modifier::empty(),
        "the text's own weight, slant, decorations"
    );
    assert_eq!(cell.underline_color, Color::Reset);
}

/// The same for a border glyph: a positioned box's border, in the box's
/// default colour, drawn over red bold text shows in its own colour and
/// unstyled (CSS Backgrounds 3 §4.1: `border-color` is `currentcolor`).
#[test]
fn a_border_painted_over_text_is_drawn_in_its_own_color() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "class", "t").unwrap();
    dom.append_child(root, p).unwrap();
    let t = dom.create_text_node("XXXXXX");
    dom.append_child(p, t).unwrap();
    let b = dom.create_element("div");
    dom.set_attribute(b, "class", "b").unwrap();
    dom.append_child(root, b).unwrap();
    let sheet = rdom_css::from_css(
        ".t { color: rgb(192, 0, 0); font-weight: bold; text-decoration: underline }
         .b { position: absolute; left: 0; top: 0; width: 4; height: 1; border: solid }",
    );
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 8, 3));
    let cell = buf.cell(1, 0).unwrap();
    assert_eq!(cell.symbol(), "─");
    assert_eq!(cell.fg, Color::Reset, "the border's currentcolor");
    assert_eq!(cell.modifier, Modifier::empty());
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Reset);
}
