//! `filter` and `backdrop-filter` (Filter Effects 1 §5–§6; Filter
//! Effects 2 §3, C15-FILTER): the color-matrix functions applied, in sRGB,
//! to the colors of every cell the element and its descendants paint — the
//! cells they leave keep the backdrop's — `opacity()` as group opacity,
//! `drop-shadow()` as a whole-cell shade, `blur()` and `url()` inert.

use super::*;

const BLACK: Color = Color::Rgb(0, 0, 0);
const WHITE: Color = Color::Rgb(255, 255, 255);

fn bg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).unwrap().bg
}

fn fg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).unwrap().fg
}

/// §6 `invert()`: every color the element paints, background and text,
/// its own and its descendants' — and only those: the backdrop's cells
/// beside it, and under its text, keep their colors. A black box on a
/// black page inverts although it changed no cell's color (the paint is
/// the element's, not the difference).
#[test]
fn invert_filters_the_element_and_its_descendants_cells_only() {
    let mut dom = doc(r#"<body><div id="f">ab<span id="c">cd</span></div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: rgb(0, 0, 0); margin: 0 }
         #f { width: 6; background-color: rgb(0, 0, 0); color: rgb(255, 255, 255); filter: invert(1) }
         #c { background-color: rgb(255, 0, 0) }",
        8,
        2,
    );
    assert_eq!((bg(&buf, 0, 0), fg(&buf, 0, 0)), (WHITE, BLACK));
    assert_eq!(bg(&buf, 2, 0), Color::Rgb(0, 255, 255), "the child's red");
    assert_eq!(bg(&buf, 7, 0), BLACK, "the page beside it");
    assert_eq!(bg(&buf, 0, 1), BLACK, "the page below it");
    // Text without a background of its own: the glyph is the element's,
    // the background under it the page's.
    let mut dom = doc(r#"<body><div id="f">ab</div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: rgb(0, 0, 200); margin: 0 }
         #f { color: rgb(255, 255, 255); filter: invert(100%) }",
        4,
        1,
    );
    assert_eq!(
        (bg(&buf, 0, 0), fg(&buf, 0, 0)),
        (Color::Rgb(0, 0, 200), BLACK)
    );
}

/// §6's matrices, each in sRGB (§5: filter functions operate in sRGB),
/// one rounding to 8-bit channels.
#[test]
fn the_color_matrix_functions_follow_their_matrices() {
    for (filter, color, want) in [
        ("grayscale(1)", "rgb(255, 0, 0)", Color::Rgb(54, 54, 54)),
        ("sepia(1)", "rgb(100, 150, 201)", Color::Rgb(193, 172, 134)),
        ("saturate(0.5)", "rgb(200, 50, 50)", Color::Rgb(141, 66, 66)),
        (
            "hue-rotate(180deg)",
            "rgb(255, 0, 0)",
            Color::Rgb(0, 109, 109),
        ),
        (
            "brightness(0.5)",
            "rgb(200, 100, 50)",
            Color::Rgb(100, 50, 25),
        ),
        (
            "contrast(0.5)",
            "rgb(200, 100, 50)",
            Color::Rgb(164, 114, 89),
        ),
        ("invert(0.5)", "rgb(0, 100, 255)", Color::Rgb(128, 128, 128)),
        (
            "grayscale(1) invert(1)",
            "rgb(255, 0, 0)",
            Color::Rgb(201, 201, 201),
        ),
        ("blur(3px) url(#x)", "rgb(1, 2, 3)", Color::Rgb(1, 2, 3)),
    ] {
        let mut dom = doc(r#"<div id="f">x</div>"#);
        let buf = paint(
            &mut dom,
            &format!("#f {{ width: 2; background-color: {color}; filter: {filter} }}"),
            4,
            1,
        );
        assert_eq!(bg(&buf, 0, 0), want, "{filter} of {color}");
    }
}

/// §6 `opacity()` is the group opacity `opacity` is: the same cells.
#[test]
fn the_opacity_function_is_group_opacity() {
    let markup = r#"<body><div id="f">ab</div></body>"#;
    let base = "body { background-color: rgb(0, 0, 200); margin: 0 }
                #f { width: 3; background-color: rgb(200, 0, 0); color: rgb(255, 255, 255) }";
    let mut a = doc(markup);
    let filtered = paint(
        &mut a,
        &format!("{base} #f {{ filter: opacity(0.5) }}"),
        5,
        1,
    );
    let mut b = doc(markup);
    let opaque = paint(&mut b, &format!("{base} #f {{ opacity: 0.5 }}"), 5, 1);
    assert_eq!(filtered, opaque);
    assert_ne!(bg(&filtered, 0, 0), Color::Rgb(200, 0, 0));
}

/// §6 `drop-shadow()`: the element's painted cells, offset, shade the
/// cells beneath it it does not paint itself — box-shadow's whole-cell
/// shade.
#[test]
fn drop_shadow_shades_the_offset_cells() {
    let mut dom = doc(r#"<div id="f">ab</div>"#);
    let buf = paint(
        &mut dom,
        "#f { width: 2; height: 1; background-color: rgb(255, 0, 0);
              filter: drop-shadow(rgb(0, 0, 255) 1 1) }",
        5,
        3,
    );
    assert_eq!(bg(&buf, 0, 0), RED);
    assert_eq!(bg(&buf, 1, 0), RED, "the element over its own shadow");
    assert_eq!(bg(&buf, 1, 1), BLUE);
    assert_eq!(bg(&buf, 2, 1), BLUE);
    assert_ne!(bg(&buf, 0, 1), BLUE);
    assert_ne!(bg(&buf, 3, 1), BLUE);
}

/// §5: a `filter` other than `none` makes a stacking context and the
/// containing block of absolute and fixed descendants — `blur()` too,
/// though it draws nothing.
#[test]
fn a_filter_makes_a_stacking_context_and_a_containing_block() {
    for filter in ["filter: blur(1px)", "backdrop-filter: invert(0)"] {
        let mut dom = doc(r#"<div id="t"><div id="k">k</div></div>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "#t {{ {filter}; background-color: rgb(0, 0, 255); height: 2 }}
                 #k {{ position: relative; z-index: -1; background-color: rgb(255, 0, 0); width: 1; height: 1 }}"
            ),
            6,
            3,
        );
        assert_eq!(bg(&buf, 0, 0), RED, "{filter}");
        let mut dom = doc(r#"<p>x</p><div id="t"><span id="f">f</span></div>"#);
        styled(
            &mut dom,
            &format!(
                "p {{ margin: 0 }} #t {{ {filter}; margin-left: 4; height: 3 }}
                 #f {{ position: fixed; top: 0; left: 1 }}"
            ),
            30,
            8,
        );
        assert_eq!((rect(&dom, "f").0, rect(&dom, "f").1), (5, 1), "{filter}");
    }
}

/// Filter Effects 2 §3: `backdrop-filter` filters what is painted behind
/// the element's border box, before the element paints; the element's own
/// paint is not filtered.
#[test]
fn backdrop_filter_filters_the_cells_behind_the_border_box() {
    let mut dom = doc(r#"<body><p>xxxxxx</p><div id="f">ab</div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: rgb(0, 0, 0); color: rgb(0, 255, 0); margin: 0 } p { margin: 0 }
         #f { position: absolute; top: 0; left: 1; width: 3; color: rgb(255, 0, 0);
              backdrop-filter: invert(1) }",
        8,
        2,
    );
    assert_eq!(bg(&buf, 0, 0), BLACK, "outside the box");
    assert_eq!(bg(&buf, 1, 0), WHITE, "behind the box");
    assert_eq!(bg(&buf, 3, 0), WHITE);
    assert_eq!(fg(&buf, 1, 0), RED, "the element's own text, unfiltered");
    assert_eq!(
        fg(&buf, 3, 0),
        Color::Rgb(255, 0, 255),
        "the page's text behind it, inverted"
    );
    assert_eq!(bg(&buf, 4, 0), BLACK);
}

/// A default-colored glyph (`Color::Reset`) filters as the canvas text of
/// the color scheme — white under dark — so `invert()` draws it black.
#[test]
fn default_colors_filter_as_the_canvas() {
    let mut dom = doc(r#"<div id="f">ab</div>"#);
    let buf = paint(&mut dom, "#f { filter: invert(1) }", 4, 1);
    assert_eq!(fg(&buf, 0, 0), BLACK);
}

/// §5: the filter maps the element's image — its descendants' paint
/// included, through every nested group. A child at `opacity: .9` paints
/// black on the black page into its own layer; composited into the
/// filter's layer it changes no cell's color, but it was painted, so
/// `invert(1)` draws it white (C15G-FILTER-COVERAGE).
#[test]
fn a_filter_maps_what_a_nested_group_painted() {
    let mut dom = doc(r#"<body><div class="f"><i>ab</i></div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: #000; margin: 0 }
         .f { filter: invert(1) }
         .f > i { color: #fff; display: block; width: 2; opacity: .9; background-color: #000 }",
        6,
        1,
    );
    assert_eq!(bg(&buf, 0, 0), WHITE, "the child's black, inverted");
    // White text at .9 over the black: 230, then inverted.
    assert_eq!(
        fg(&buf, 0, 0),
        Color::Rgb(25, 25, 25),
        "the child's text, inverted"
    );
    assert_eq!(bg(&buf, 4, 0), BLACK, "the page beside it, unpainted");
}

/// The same through a translucent paint's scratch layer: a half-black
/// background on the black page blends to black first (DIVERGENCES: a
/// translucent paint blends before the filter), and that cell is the
/// element's paint, so it inverts (C15G-FILTER-COVERAGE).
#[test]
fn a_filter_maps_what_a_translucent_paint_painted() {
    let mut dom = doc(r#"<body><div class="f">ab</div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: #000; margin: 0 }
         .f { width: 2; filter: invert(1); background-color: rgb(0 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(bg(&buf, 0, 0), WHITE);
    assert_eq!(bg(&buf, 4, 0), BLACK);
}
