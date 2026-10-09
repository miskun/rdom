//! CSS-COMPLETE Phase 4 gate — the border edge cases the gates asked for
//! (C4G-EDGE-TESTS): each sheet parsed strictly, cascaded, laid out and
//! painted, asserting painted cells. Each test cites the spec text that
//! fixes the expected cells.

use rdom_tui::render::{Buffer, Cell, Rect};
use rdom_tui::{CascadeExt, Color, LayoutExt, NodeId, PaintExt, TuiDom};

const RED: Color = Color::Rgb(255, 0, 0);

fn el(dom: &mut TuiDom, parent: NodeId, class: &str) -> NodeId {
    let id = dom.create_element("div");
    dom.set_attribute(id, "class", class).unwrap();
    dom.append_child(parent, id).unwrap();
    id
}

fn paint(dom: &mut TuiDom, css: &str, w: u16, h: u16) -> Buffer {
    let sheet = rdom_css::from_css_strict(&crate::common::border_box(css))
        .expect("sheet parses without warnings");
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

/// A 5 × 3 `div.b` painted under `css`.
fn bordered(css: &str) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b");
    paint(&mut dom, css, 5, 3)
}

/// The glyphs of a `w` × `h` region, row by row.
fn rows(buf: &Buffer, w: u16, h: u16) -> Vec<String> {
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| cell(buf, x, y).symbol().to_string())
                .collect()
        })
        .collect()
}

// ── An opaque side meets a translucent one ─────────────────────────

/// CSS Backgrounds 3 §4.1 with CSS Color 4 §4.2: an opaque top / bottom
/// and translucent left / right. The corner cells go to the horizontal
/// side (equal width and style; DIVERGENCES §2), so they are the opaque
/// side's glyph and color, whole; the translucent sides blend over the
/// backdrop on their own cells only and leave the corners alone.
#[test]
fn an_opaque_side_meets_a_translucent_side_at_a_corner() {
    let buf = bordered(
        ".b { width: 5; height: 3; background-color: rgb(0, 0, 0); border: solid; \
              border-color: red rgba(0, 0, 255, 0.5) }",
    );
    assert_eq!(rows(&buf, 5, 3), ["┌───┐", "│   │", "└───┘"]);
    for (x, y) in [(0, 0), (4, 0), (0, 2), (4, 2), (2, 0), (2, 2)] {
        assert_eq!(cell(&buf, x, y).fg, RED, "opaque cell ({x}, {y})");
    }
    for x in [0, 4] {
        let fg = cell(&buf, x, 1).fg;
        assert_eq!(fg, Color::Rgb(0, 0, 128), "translucent side at x={x}");
    }
}

// ── `border-width` range ───────────────────────────────────────────

/// CSS Backgrounds 3 §4.3: `<line-width>` is a non-negative length; a
/// negative one makes the declaration invalid — as a literal, in every
/// unit and in the shorthands (a math function's result clamps, §10.12).
#[test]
fn a_negative_border_width_is_rejected() {
    for css in [
        ".b { border-width: -1px }",
        ".b { border-width: -1 }",
        ".b { border-top-width: -0.5em }",
        ".b { border: -1px solid }",
        ".b { border-left: solid -2 }",
    ] {
        assert!(
            rdom_css::from_css_strict(&crate::common::border_box(css)).is_err(),
            "{css} parsed"
        );
    }
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-width: calc(-1px) }");
    assert_eq!(
        rows(&buf, 5, 3),
        ["     ", "     ", "     "],
        "calc(-1px) is 0"
    );
}

// ── Percentage radii ───────────────────────────────────────────────

/// CSS Backgrounds 3 §5.1: a percentage radius is a percentage of the
/// border box on its axis. A 2 × 2 border box: `50%` and `25%` are one
/// and a half cell — non-zero, so the corners round. A zero-size box:
/// the content box is zero but the border box is floored at the border
/// (CSS UI 3 §3.1, C5-BOX-SIZING — it used to paint nothing), so the
/// percentages are of that 2-cell border box and the corners round,
/// without a panic or a NaN on the way.
#[test]
fn a_percentage_radius_against_a_zero_size_box() {
    for radius in ["50%", "25%", "50% / 25%"] {
        let buf = bordered(&format!(
            ".b {{ width: 2; height: 2; border: solid; border-radius: {radius} }}"
        ));
        assert_eq!(rows(&buf, 2, 2), ["╭╮", "╰╯"], "{radius}");
    }
    for (css, want) in [
        (
            ".b { width: 0; height: 0; border: solid; border-radius: 50% }",
            ["╭╮   ", "╰╯   ", "     "],
        ),
        (
            ".b { width: 0; height: 3; border: solid; border-radius: 50% }",
            ["╭╮   ", "││   ", "╰╯   "],
        ),
        (
            ".b { width: 5; height: 0; border: solid; border-radius: 100% / 50% }",
            ["╭───╮", "╰───╯", "     "],
        ),
    ] {
        let buf = bordered(css);
        assert_eq!(rows(&buf, 5, 3), want, "{css}");
    }
}

// ── `border: var(--b)` ─────────────────────────────────────────────

/// CSS Variables 1 §3: a `var()` is substituted, and the result parsed
/// by the property's grammar, at computed-value time — so `border:
/// var(--b)` with `--b: rounded` is `border: rounded`, radius included.
#[test]
fn border_rounded_through_var_rounds() {
    let buf = bordered(".b { --b: rounded; width: 5; height: 3; border: var(--b) }");
    assert_eq!(rows(&buf, 5, 3), ["╭───╮", "│   │", "╰───╯"]);
    let buf = bordered(".b { --b: 1px rounded red; width: 5; height: 3; border: var(--b) }");
    assert_eq!(cell(&buf, 0, 0).symbol(), "╭");
    assert_eq!(cell(&buf, 0, 0).fg, RED);
}

// ── `dashed` / `dotted` ────────────────────────────────────────────

/// CSS Backgrounds 3 §4.2: `dashed` is a series of square-ended dashes,
/// `dotted` a series of round dots. A cell draws Unicode's dash glyphs —
/// `dashed` the double dash `╌╎`, `dotted` the triple dash `┄┆`, heavy
/// forms for `thick` — on the straight runs; Unicode has no dashed
/// corner or junction, so those stay solid (DIVERGENCES §1).
#[test]
fn dashed_and_dotted_draw_dash_glyphs() {
    let buf = bordered(".b { width: 5; height: 3; border: dashed }");
    assert_eq!(rows(&buf, 5, 3), ["┌╌╌╌┐", "╎   ╎", "└╌╌╌┘"]);
    let buf = bordered(".b { width: 5; height: 3; border: dotted }");
    assert_eq!(rows(&buf, 5, 3), ["┌┄┄┄┐", "┆   ┆", "└┄┄┄┘"]);
    let buf = bordered(".b { width: 5; height: 3; border: thick dashed }");
    assert_eq!(rows(&buf, 5, 3), ["┏╍╍╍┓", "╏   ╏", "┗╍╍╍┛"]);
    let buf = bordered(".b { width: 5; height: 3; border: thick dotted }");
    assert_eq!(rows(&buf, 5, 3), ["┏┅┅┅┓", "┇   ┇", "┗┅┅┅┛"]);
    let buf = bordered(".b { width: 5; height: 3; border: dashed; border-radius: 1 }");
    assert_eq!(rows(&buf, 5, 3), ["╭╌╌╌╮", "╎   ╎", "╰╌╌╌╯"]);
    // Per side: a dashed top over solid sides.
    let buf = bordered(".b { width: 5; height: 3; border: solid; border-top-style: dashed }");
    assert_eq!(rows(&buf, 5, 3), ["┌╌╌╌┐", "│   │", "└───┘"]);
}

/// ACID-FIX-2 (acid tile 5). A side whose neighbour has no border ends in
/// a straight cell, not a corner: CSS Backgrounds 3 §4.2 runs each side
/// the box's full length, and with the bottom border `hidden` (or `none`,
/// or the top's) there is no direction change at the end cell — so a
/// dashed side draws its dash glyph there too (DIVERGENCES §2: dash glyphs
/// on straight runs, solid only at corners and junctions).
#[test]
fn a_dashed_side_ends_in_a_dash_where_no_side_meets_it() {
    let buf = bordered(".b { width: 5; height: 3; border-left: dashed; border-right: dotted }");
    assert_eq!(rows(&buf, 5, 3), ["╎   ┆", "╎   ┆", "╎   ┆"]);
    let buf = bordered(".b { width: 5; height: 3; border: dashed; border-bottom-style: hidden }");
    assert_eq!(rows(&buf, 5, 3), ["┌╌╌╌┐", "╎   ╎", "╎   ╎"]);
    let buf = bordered(".b { width: 5; height: 3; border-top: thick dashed }");
    assert_eq!(rows(&buf, 5, 3), ["╍╍╍╍╍", "     ", "     "]);
}

/// ACID-FIX-12 (found by acid tile 27). DIVERGENCES §1: Unicode has no
/// glyph where a heavy line meets a double one, so the corner's dominant
/// side — the wider, here the `thick` one — draws the whole cell in its
/// set: a heavy corner `┛`, not the light-and-heavy `┚` that drew the
/// double side as a thin single line.
#[test]
fn a_heavy_side_meeting_a_double_one_draws_the_corner_in_its_set() {
    let buf = bordered(
        ".b { width: 5; height: 3; border-right: thick solid; border-bottom: double; border-left: solid }",
    );
    assert_eq!(rows(&buf, 5, 3), ["│   ┃", "│   ┃", "╘═══┛"]);
}
