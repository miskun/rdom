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

// ── C4-BG-CLIP ─────────────────────────────────────────────────────

/// A 6 × 3 box with a solid border, one column of horizontal padding,
/// a red background and `clip` (a `background-clip` value, or `None`
/// for none declared); the buffer it paints.
fn clipped_box(clip: Option<&str>) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    let clip = clip.map_or(String::new(), |c| format!("background-clip: {c};"));
    paint(
        &mut dom,
        &format!(
            ".b {{ width: 6; height: 3; border: solid; padding: 0 1; \
                   background-color: red; {clip} }}"
        ),
        6,
        3,
    )
}

const RED: Color = Color::Rgb(255, 0, 0);

/// CSS Backgrounds 3 §3.8: `background-clip`'s initial value is
/// `border-box` — the background is painted under the border, so the
/// border cells take the box's background.
#[test]
fn background_clip_defaults_to_the_border_box() {
    for buf in [clipped_box(None), clipped_box(Some("border-box"))] {
        assert_eq!(cell(&buf, 0, 0).symbol(), "┌");
        assert_eq!(cell(&buf, 0, 0).bg, RED, "corner under the border");
        assert_eq!(cell(&buf, 3, 2).bg, RED, "bottom edge");
        assert_eq!(cell(&buf, 1, 1).bg, RED, "padding");
    }
}

/// §3.8: `padding-box` — nothing is painted under the border.
#[test]
fn background_clip_padding_box_leaves_the_border_cells() {
    let buf = clipped_box(Some("padding-box"));
    assert_eq!(cell(&buf, 0, 0).symbol(), "┌");
    assert_eq!(cell(&buf, 0, 0).bg, Color::Reset);
    assert_eq!(cell(&buf, 0, 1).bg, Color::Reset);
    assert_eq!(cell(&buf, 5, 1).bg, Color::Reset);
    assert_eq!(
        cell(&buf, 1, 1).bg,
        RED,
        "padding is inside the padding box"
    );
    assert_eq!(cell(&buf, 2, 1).bg, RED);
}

/// §3.8: `content-box` — nothing under the border or the padding.
#[test]
fn background_clip_content_box_leaves_the_padding_too() {
    let buf = clipped_box(Some("content-box"));
    assert_eq!(cell(&buf, 0, 1).bg, Color::Reset);
    assert_eq!(cell(&buf, 1, 1).bg, Color::Reset, "padding");
    assert_eq!(cell(&buf, 4, 1).bg, Color::Reset, "padding");
    assert_eq!(cell(&buf, 2, 1).bg, RED);
    assert_eq!(cell(&buf, 3, 1).bg, RED);
}

/// §3.2 / §3.8: the color is clipped by the bottom-most (final)
/// layer's `background-clip`; the shorthand's one box sets it.
#[test]
fn background_clip_of_the_final_layer_clips_the_color() {
    let buf = clipped_box(Some("content-box, border-box"));
    assert_eq!(cell(&buf, 0, 0).bg, RED);
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    let buf = paint(
        &mut dom,
        ".b { width: 6; height: 3; border: solid; padding: 0 1; background: padding-box red }",
        6,
        3,
    );
    assert_eq!(cell(&buf, 0, 1).bg, Color::Reset);
    assert_eq!(cell(&buf, 1, 1).bg, RED);
}

// ── C4-BORDER-SHORTHAND ────────────────────────────────────────────

/// A 5 × 3 `div.b` painted under `css`.
fn bordered(css: &str) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    paint(&mut dom, css, 5, 3)
}

/// CSS Backgrounds 3 §4.4: `border: 1px solid red` — the most common
/// border declaration — draws a solid red ring.
#[test]
fn border_shorthand_with_width_and_color_draws_a_colored_ring() {
    let buf = bordered(".b { width: 5; height: 3; border: 1px solid red }");
    assert_eq!(cell(&buf, 0, 0).symbol(), "┌");
    assert_eq!(cell(&buf, 2, 0).symbol(), "─");
    assert_eq!(cell(&buf, 0, 1).symbol(), "│");
    for (x, y) in [(0, 0), (2, 0), (0, 1), (4, 1), (2, 2), (4, 2)] {
        assert_eq!(cell(&buf, x, y).fg, RED, "({x}, {y})");
    }
}

/// §4.4: `border-top: 1px solid red` after `border: solid blue` colors
/// the top side only.
#[test]
fn border_side_shorthand_colors_its_side_only() {
    let buf = bordered(".b { width: 5; height: 3; border: solid blue; border-top: 1px solid red }");
    let blue = Color::Rgb(0, 0, 255);
    assert_eq!(cell(&buf, 2, 0).fg, RED, "top edge");
    assert_eq!(cell(&buf, 2, 2).fg, blue, "bottom edge");
    assert_eq!(cell(&buf, 0, 1).fg, blue, "left edge");
    assert_eq!(cell(&buf, 4, 1).fg, blue, "right edge");
}

// ── C4-BORDER-SIDES ────────────────────────────────────────────────

const BLUE: Color = Color::Rgb(0, 0, 255);

/// CSS Backgrounds 3 §4.1: `border-color: red blue` — top and bottom
/// red, left and right blue. A corner cell joins two sides and can show
/// one color: the browser splits a corner between them, the wider side
/// taking more; at equal width and style rdom gives every corner to
/// its horizontal side (DIVERGENCES §2), so the top and bottom read as
/// whole lines.
#[test]
fn per_side_colors_and_the_corner_rule() {
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-color: red blue }");
    assert_eq!(cell(&buf, 2, 0).fg, RED, "top edge");
    assert_eq!(cell(&buf, 2, 2).fg, RED, "bottom edge");
    assert_eq!(cell(&buf, 0, 1).fg, BLUE, "left edge");
    assert_eq!(cell(&buf, 4, 1).fg, BLUE, "right edge");
    for (x, y) in [(0, 0), (4, 0), (0, 2), (4, 2)] {
        assert_eq!(cell(&buf, x, y).fg, RED, "corner ({x}, {y})");
    }
}

/// The dominant side of a corner is the heavier style first (CSS
/// Tables 3 §11.5's ranking, `double` above `solid`): a double left
/// side owns its corners — glyph and color.
#[test]
fn a_corner_goes_to_the_dominant_style() {
    let buf = bordered(".b { width: 5; height: 3; border: solid red; border-left: double blue }");
    assert_eq!(cell(&buf, 0, 0).fg, BLUE);
    assert_eq!(cell(&buf, 0, 2).fg, BLUE);
    assert_eq!(cell(&buf, 4, 0).fg, RED);
}

/// §4.1: each side's color is its own longhand in the cascade — a later
/// rule's `border-left-color` leaves the other sides of an earlier
/// `border-color`.
#[test]
fn a_side_color_longhand_cascades_alone() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "b", "");
    dom.set_attribute(b, "id", "x").unwrap();
    let buf = paint(
        &mut dom,
        ".b { width: 5; height: 3; border: solid; border-color: red } \
         #x { border-left-color: blue }",
        5,
        3,
    );
    assert_eq!(cell(&buf, 0, 1).fg, BLUE);
    assert_eq!(cell(&buf, 4, 1).fg, RED);
    assert_eq!(cell(&buf, 2, 0).fg, RED);
}
