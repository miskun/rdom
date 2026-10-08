//! CSS-COMPLETE Phase 3 colors end to end: a sheet parsed by
//! `rdom_css::from_css_strict`, cascaded, laid out and painted by
//! `rdom-tui`, asserting the painted cells' colors. One test per color
//! form; each cites the spec text that fixes the expected value.

use rdom_tui::render::{Buffer, Cell, Rect};
use rdom_tui::{CascadeExt, Color, ColorScheme, LayoutExt, NodeId, PaintExt, TuiDom};

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

/// Cascade `css` (no warning allowed) under the document's preferred
/// `scheme`, lay out and paint into an 8 × 3 buffer.
fn paint(dom: &mut TuiDom, css: &str, scheme: ColorScheme) -> Buffer {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.set_color_scheme(scheme);
    dom.cascade(&sheet);
    let area = Rect::new(0, 0, 8, 3);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

fn cell(buf: &Buffer, x: u16, y: u16) -> &Cell {
    buf.cell(x, y).expect("in the buffer")
}

/// `c` is within `tol` of `(r, g, b)` on every channel (conversions
/// through Oklab round).
fn near(c: Color, (r, g, b): (u8, u8, u8), tol: u8) -> bool {
    match c {
        Color::Rgb(cr, cg, cb) => {
            cr.abs_diff(r) <= tol && cg.abs_diff(g) <= tol && cb.abs_diff(b) <= tol
        }
        _ => false,
    }
}

/// CSS Color 4 §9.4 / §13.2: `oklch()` converts to sRGB — white at 100%
/// lightness, and the Oklch of sRGB red back to red.
#[test]
fn oklch_paints_its_srgb_color() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "w", "w");
    el(&mut dom, root, "r", "r");
    let buf = paint(
        &mut dom,
        ".w { color: oklch(100% 0 0) } .r { color: oklch(62.8% 0.2577 29.23) }",
        ColorScheme::Dark,
    );
    assert_eq!(cell(&buf, 0, 0).symbol(), "w");
    assert_eq!(cell(&buf, 0, 0).fg, Color::Rgb(255, 255, 255));
    let red = cell(&buf, 0, 1).fg;
    assert!(near(red, (255, 0, 0), 2), "{red:?}");
}

/// CSS Color 4 §4.2 / §5.1: `rgb(0 0 0 / 50%)` is black at half alpha,
/// composited over the parent's background — `(200, 0, 0)` beneath gives
/// `(100, 0, 0)`; beside the child the parent's color is untouched.
#[test]
fn a_translucent_background_composites_over_its_parent() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    el(&mut dom, p, "c", "");
    let buf = paint(
        &mut dom,
        ".p { background-color: rgb(200 0 0); width: 4; height: 1 } \
         .c { background-color: rgb(0 0 0 / 50%); width: 2; height: 1 }",
        ColorScheme::Dark,
    );
    let under = cell(&buf, 0, 0).bg;
    assert!(near(under, (100, 0, 0), 1), "{under:?}");
    assert_eq!(cell(&buf, 3, 0).bg, Color::Rgb(200, 0, 0));
}

/// CSS Color 4 §6.4: `currentColor` in `border-color` is the element's
/// `color`.
#[test]
fn a_current_color_border_takes_the_text_color() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    let buf = paint(
        &mut dom,
        ".b { color: rgb(0 128 0); border: solid; border-color: currentColor; \
              width: 4; height: 3; box-sizing: border-box }",
        ColorScheme::Dark,
    );
    let corner = cell(&buf, 0, 0);
    assert_ne!(corner.symbol().trim(), "", "a border glyph");
    assert_eq!(corner.fg, Color::Rgb(0, 128, 0));
    assert_eq!(cell(&buf, 3, 2).fg, Color::Rgb(0, 128, 0));
}

/// CSS Color 5 §2: `color-mix()` in sRGB, 50% each, is the channel
/// midpoint — and with `currentColor` it mixes the element's color.
#[test]
fn color_mix_paints_the_mixture() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "m", "m");
    el(&mut dom, root, "n", "n");
    let buf = paint(
        &mut dom,
        ".m { color: color-mix(in srgb, rgb(255 0 0), rgb(0 0 255)) } \
         .n { color: rgb(0 0 200); background-color: color-mix(in srgb, currentColor, white) }",
        ColorScheme::Dark,
    );
    let mixed = cell(&buf, 0, 0).fg;
    assert!(near(mixed, (128, 0, 128), 1), "{mixed:?}");
    let bg = cell(&buf, 0, 1).bg;
    assert!(near(bg, (128, 128, 228), 1), "{bg:?}");
}

/// CSS Color 5 §5.1, Color Adjust 1 §2.1: `light-dark()` takes the arm of
/// the element's used scheme — the document's preferred one under
/// `color-scheme: normal`, light under `color-scheme: light` whatever
/// the document prefers.
#[test]
fn light_dark_follows_the_scheme() {
    for (scheme, normal) in [
        (ColorScheme::Dark, Color::Rgb(4, 5, 6)),
        (ColorScheme::Light, Color::Rgb(1, 2, 3)),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        el(&mut dom, root, "a", "a");
        el(&mut dom, root, "l", "l");
        let buf = paint(
            &mut dom,
            ".a, .l { color: light-dark(rgb(1 2 3), rgb(4 5 6)) } .l { color-scheme: light }",
            scheme,
        );
        assert_eq!(cell(&buf, 0, 0).fg, normal, "{scheme:?}");
        assert_eq!(cell(&buf, 0, 1).fg, Color::Rgb(1, 2, 3), "{scheme:?}");
    }
}

/// CSS Color 5 §4 with CSS Variables 1 §3: a relative color whose origin
/// is a custom property — the channels bound to the origin's, math on
/// them, the alpha replaced.
#[test]
fn a_relative_color_through_var_paints() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "r", "r");
    let buf = paint(
        &mut dom,
        ":root { --brand: rgb(10 20 30) } \
         .r { color: rgb(from var(--brand) calc(r + 10) g b) }",
        ColorScheme::Dark,
    );
    assert_eq!(cell(&buf, 0, 0).fg, Color::Rgb(20, 20, 30));
}

/// CSS Color 4 §6.2: `CanvasText` / `Canvas` are the terminal's own
/// default colors — painted as the terminal default (`reset`), not a
/// fixed RGB, in either scheme.
#[test]
fn canvas_text_paints_the_terminal_default() {
    for scheme in [ColorScheme::Dark, ColorScheme::Light] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        el(&mut dom, root, "k", "k");
        let buf = paint(
            &mut dom,
            ".k { color: CanvasText; background-color: Canvas }",
            scheme,
        );
        let k = cell(&buf, 0, 0);
        assert_eq!(k.symbol(), "k");
        assert_eq!((k.fg, k.bg), (Color::Reset, Color::Reset), "{scheme:?}");
    }
}

/// CSS Color 4 §6.3: text in `transparent` paints no glyph — what is
/// beneath shows (here the parent's blue background, no text).
#[test]
fn transparent_text_paints_nothing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    el(&mut dom, p, "t", "xy");
    let buf = paint(
        &mut dom,
        ".p { background-color: rgb(0 0 255); width: 4; height: 1 } .t { color: transparent }",
        ColorScheme::Dark,
    );
    for x in 0..2 {
        let c = cell(&buf, x, 0);
        assert_eq!(c.symbol().trim(), "", "no glyph at {x}");
        assert_eq!(c.bg, Color::Rgb(0, 0, 255));
    }
}
