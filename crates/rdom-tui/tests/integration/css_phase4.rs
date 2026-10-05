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

// ── C4-BORDER-WIDTH ────────────────────────────────────────────────

/// The glyphs of a 5 × 3 box's border, row by row.
fn ring(buf: &Buffer) -> Vec<String> {
    (0..3)
        .map(|y| {
            (0..5)
                .map(|x| cell(buf, x, y).symbol().to_string())
                .collect()
        })
        .collect()
}

/// CSS Backgrounds 3 §4.3: `thin` ≤ `medium` ≤ `thick`. A border is one
/// cell wide (DIVERGENCES §2); `thin` and `medium` (the initial value)
/// draw the light box-drawing set, `thick` the heavy set.
#[test]
fn thick_borders_draw_heavy_glyphs() {
    let light = ["┌───┐", "│   │", "└───┘"];
    let heavy = ["┏━━━┓", "┃   ┃", "┗━━━┛"];
    for (width, glyphs) in [("thin", light), ("medium", light), ("thick", heavy)] {
        let buf = bordered(&format!(
            ".b {{ width: 5; height: 3; border: {width} solid }}"
        ));
        assert_eq!(ring(&buf), glyphs, "{width}");
    }
}

/// Lengths map onto the same two weights: below `thick` (5px; two
/// cells) light, from it on heavy.
#[test]
fn border_width_lengths_pick_a_weight() {
    for (width, corner) in [
        ("1px", "┌"),
        ("4px", "┌"),
        ("5px", "┏"),
        ("12px", "┏"),
        ("1", "┌"),
        ("2", "┏"),
        ("0.25em", "┌"),
    ] {
        let buf = bordered(&format!(
            ".b {{ width: 5; height: 3; border: {width} solid }}"
        ));
        assert_eq!(cell(&buf, 0, 0).symbol(), corner, "{width}");
    }
}

/// §4.3: a side whose width is 0 has no border — it takes no cell and
/// draws nothing, whatever its style.
#[test]
fn a_zero_width_side_has_no_border() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "x");
    let buf = paint(
        &mut dom,
        ".b { width: 5; height: 3; border: 0 solid red }",
        5,
        3,
    );
    assert_eq!(
        cell(&buf, 0, 0).symbol(),
        "x",
        "content at the border box's corner"
    );
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-width: 0 1 }");
    assert_eq!(ring(&buf), ["│   │", "│   │", "│   │"]);
}

/// Sides of different weights meet in the mixed junction glyphs: a
/// heavy top over light sides turns its corners `┍` / `┑`.
#[test]
fn mixed_weights_meet_in_mixed_corners() {
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-top-width: thick }");
    assert_eq!(ring(&buf), ["┍━━━┑", "│   │", "└───┘"]);
}

/// §4.3: the computed `border-style` is not changed by a zero width —
/// only the used border is — so a child inheriting the style takes
/// `solid` and, with its own width, draws it.
#[test]
fn a_zero_width_keeps_the_computed_style() {
    use rdom_tui::style::cascade::computed_of;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let c = el(&mut dom, p, "c", "");
    let sheet = rdom_css::from_css_strict(
        ".p { border: 0 solid } .c { border-style: inherit; width: 3; height: 3 }",
    )
    .unwrap();
    dom.cascade(&sheet);
    let parent = computed_of(&dom, p);
    assert!(parent.border.is_empty(), "the used border has no side");
    assert_eq!(
        parent.border_style.top,
        rdom_tui::layout::BorderStyle::Solid
    );
    let child = computed_of(&dom, c);
    assert_eq!(child.border.top, rdom_tui::layout::BorderStyle::Solid);
}

// ── C4-RADIUS ──────────────────────────────────────────────────────

/// CSS Backgrounds 3 §5.1: a non-zero `border-radius` rounds the corner;
/// a terminal's rounded corner is the arc glyph `╭╮╰╯`.
#[test]
fn border_radius_rounds_the_corners() {
    for radius in ["1", "4px", "50%", "0.5em", "1 / 2"] {
        let buf = bordered(&format!(
            ".b {{ width: 5; height: 3; border: solid; border-radius: {radius} }}"
        ));
        assert_eq!(ring(&buf), ["╭───╮", "│   │", "╰───╯"], "{radius}");
    }
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-radius: 0 }");
    assert_eq!(ring(&buf), ["┌───┐", "│   │", "└───┘"]);
}

/// §5.1: each corner has its own radius, and a corner with either
/// radius zero is square.
#[test]
fn each_corner_rounds_on_its_own() {
    let buf = bordered(
        ".b { width: 5; height: 3; border: solid; border-top-left-radius: 4px; \
              border-bottom-right-radius: 3px 0 }",
    );
    assert_eq!(ring(&buf), ["╭───┐", "│   │", "└───┘"]);
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-radius: 0 1 }");
    assert_eq!(ring(&buf), ["┌───╮", "│   │", "╰───┘"]);
}

/// Unicode has no heavy arc: a thick rounded corner stays square.
#[test]
fn a_heavy_rounded_corner_is_square() {
    let buf = bordered(".b { width: 5; height: 3; border: thick solid; border-radius: 1 }");
    assert_eq!(cell(&buf, 0, 0).symbol(), "┏");
}

/// rdom's `border: rounded` is `border: solid` with `border-radius: 1`.
#[test]
fn border_rounded_is_solid_with_a_radius() {
    let buf = bordered(".b { width: 5; height: 3; border: rounded }");
    assert_eq!(ring(&buf), ["╭───╮", "│   │", "╰───╯"]);
}

/// §4.2: each side's style is its own longhand in the cascade — a later
/// rule's `border-top: none` removes the top and leaves the others.
#[test]
fn a_side_style_longhand_cascades_alone() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "b", "");
    dom.set_attribute(b, "id", "x").unwrap();
    let buf = paint(
        &mut dom,
        ".b { width: 5; height: 3; border: solid } #x { border-top: none }",
        5,
        3,
    );
    assert_eq!(ring(&buf), ["│   │", "│   │", "└───┘"]);
}

// ── C4-SHADOW ──────────────────────────────────────────────────────

/// A `w` × `h` `div.b` at the origin of a 6 × 5 buffer, under `css`.
fn shadowed(css: &str) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "");
    paint(&mut dom, css, 6, 5)
}

/// The cells of `buf` whose background is `c`, row by row.
fn cells_with_bg(buf: &Buffer, c: Color) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if cell(buf, x, y).bg == c {
                out.push((x, y));
            }
        }
    }
    out
}

/// CSS Backgrounds 3 §6.1: an outer shadow is the border box offset by
/// the shadow's offsets, in its color, drawn outside the border box
/// only — not under the (here transparent) box itself.
#[test]
fn box_shadow_offsets_a_shade_outside_the_box() {
    let buf = shadowed(".b { width: 3; height: 2; box-shadow: 1 1 red }");
    assert_eq!(cells_with_bg(&buf, RED), [(3, 1), (1, 2), (2, 2), (3, 2)]);
}

/// Pixel offsets keep their direction as one cell (DIVERGENCES §2) and
/// the blur radius has no effect: the web's `0 1px 3px` shadow is a
/// one-row shade under the box.
#[test]
fn box_shadow_pixel_offsets_are_one_cell() {
    let buf = shadowed(".b { width: 3; height: 2; box-shadow: 0 1px 3px red }");
    assert_eq!(cells_with_bg(&buf, RED), [(0, 2), (1, 2), (2, 2)]);
}

/// §6.1: the spread distance grows the shadow on every side — `0 0 0
/// 1px` is a one-cell ring around the box.
#[test]
fn box_shadow_spread_grows_the_shade() {
    let buf = shadowed(".b { margin: 1; width: 2; height: 1; box-shadow: 0 0 0 1 red }");
    assert_eq!(
        cells_with_bg(&buf, RED),
        [
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (0, 1),
            (3, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2)
        ]
    );
}

/// §6.1: an `inset` shadow is drawn inside the padding box, above the
/// background: the padding box minus itself offset by the shadow.
#[test]
fn box_shadow_inset_shades_inside_the_padding_box() {
    let buf = shadowed(
        ".b { width: 4; height: 3; border: solid; background-color: blue; \
              box-shadow: inset 1 1 red }",
    );
    // Border box 4 × 3: padding box (1, 1)–(2, 1). Offset 1, 1 leaves
    // its first row and column in shade.
    assert_eq!(cells_with_bg(&buf, RED), [(1, 1), (2, 1)]);
    let buf =
        shadowed(".b { width: 3; height: 3; background-color: blue; box-shadow: inset 1 1 red }");
    assert_eq!(
        cells_with_bg(&buf, RED),
        [(0, 0), (1, 0), (2, 0), (0, 1), (0, 2)]
    );
}

/// §6.1: several shadows paint front to back — the first on top.
#[test]
fn box_shadows_layer_with_the_first_on_top() {
    let buf = shadowed(".b { width: 2; height: 1; box-shadow: 1 0 red, 2 0 blue }");
    assert_eq!(cell(&buf, 2, 0).bg, RED);
    assert_eq!(cell(&buf, 3, 0).bg, BLUE);
}

/// §6.1: an omitted color is `currentcolor`; a translucent one
/// composites over what lies beneath (CSS Color 4 §4.2).
#[test]
fn box_shadow_colors() {
    let buf = shadowed(".b { width: 2; height: 1; color: rgb(0, 128, 0); box-shadow: 1 0 }");
    assert_eq!(cell(&buf, 2, 0).bg, Color::Rgb(0, 128, 0));
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    el(&mut dom, p, "c", "");
    let buf = paint(
        &mut dom,
        ".p { width: 6; height: 3; background-color: rgb(200, 0, 0) } \
         .c { width: 2; height: 1; box-shadow: 1 0 rgb(0 0 0 / 50%) }",
        6,
        3,
    );
    let shade = cell(&buf, 2, 0).bg;
    assert!(
        matches!(shade, Color::Rgb(r, 0, 0) if r.abs_diff(100) <= 1),
        "{shade:?}"
    );
    assert_eq!(cell(&buf, 3, 0).bg, Color::Rgb(200, 0, 0));
}

/// `C4G-SHADOW-CLAMP`: offsets and a spread far past the grid clamp to a
/// range the shadow geometry cannot overflow — `i32::MAX` cells (the
/// largest integer token) and `9999999999ch` panicked in debug builds.
/// A huge spread covers everything outside the box; a huge offset moves
/// the shade off the grid.
#[test]
fn box_shadow_huge_lengths_do_not_overflow() {
    for big in ["2147483647", "9999999999ch"] {
        let buf = shadowed(&format!(
            ".b {{ margin: 1; width: 2; height: 1; box-shadow: 0 0 0 {big} red }}"
        ));
        assert_eq!(cells_with_bg(&buf, RED).len(), 6 * 5 - 2, "{big}");
        let buf = shadowed(&format!(
            ".b {{ width: 2; height: 1; box-shadow: {big} 0 red }}"
        ));
        assert_eq!(cells_with_bg(&buf, RED), [], "{big}");
        // Inset: a huge spread shrinks the hole to nothing, so the whole
        // padding box is in shade; a huge offset moves the hole away.
        let buf = shadowed(&format!(
            ".b {{ width: 2; height: 1; \
                  box-shadow: inset 0 0 0 {big} red, inset {big} 0 blue }}"
        ));
        assert_eq!(cells_with_bg(&buf, RED), [(0, 0), (1, 0)], "{big}");
    }
}

// ── C4-SPACING ─────────────────────────────────────────────────────

/// CSS 2.1 §17.6.1: `border-spacing` is inherited, and computes to two
/// absolute lengths. (It spaces a separated-borders table's cells; that
/// layout lands with the table formatting context, C13-TFC.)
#[test]
fn border_spacing_computes_and_inherits() {
    use rdom_tui::layout::GapValue;
    use rdom_tui::style::cascade::computed_of;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "t", "");
    let c = el(&mut dom, t, "c", "");
    let sheet = rdom_css::from_css_strict(".t { border-spacing: 2 1 }").unwrap();
    dom.cascade(&sheet);
    for id in [t, c] {
        let s = computed_of(&dom, id).border_spacing;
        assert_eq!(
            (s.horizontal, s.vertical),
            (GapValue::Cells(2), GapValue::Cells(1))
        );
    }
}
