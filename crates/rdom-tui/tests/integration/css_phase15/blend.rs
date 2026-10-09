//! `mix-blend-mode`, `isolation` and `background-blend-mode` (Compositing
//! and Blending 1 §3, §5, §10, C15-BLEND): the colors of the cells an
//! element paints blended with its backdrop's — a glyph's with the
//! backdrop's background — within the nearest isolated group.

use super::*;

const PAGE: &str = "body { background-color: rgb(200, 100, 50); margin: 0 } ";

fn bg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).unwrap().bg
}

/// §10: every separable and non-separable mode, and Compositing 2's
/// `plus-darker` / `plus-lighter`, on a background over the page's — the
/// sRGB formulas with opaque colors, one rounding.
#[test]
fn every_blend_mode_follows_its_formula() {
    for (mode, want) in [
        ("normal", (100, 150, 200)),
        ("multiply", (78, 59, 39)),
        ("screen", (222, 191, 211)),
        ("overlay", (188, 118, 78)),
        ("darken", (100, 100, 50)),
        ("lighten", (200, 150, 200)),
        ("color-dodge", (255, 243, 232)),
        ("color-burn", (115, 0, 0)),
        ("hard-light", (157, 127, 167)),
        ("soft-light", (191, 111, 86)),
        ("difference", (100, 50, 150)),
        ("exclusion", (143, 132, 172)),
        ("hue", (64, 139, 214)),
        ("saturation", (175, 108, 75)),
        ("color", (84, 134, 184)),
        ("luminosity", (216, 116, 66)),
        ("plus-lighter", (255, 250, 250)),
        ("plus-darker", (45, 0, 0)),
    ] {
        let mut dom = doc(r#"<body><div id="b">x</div></body>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "{PAGE} #b {{ width: 2; background-color: rgb(100, 150, 200); mix-blend-mode: {mode} }}"
            ),
            4,
            2,
        );
        assert_eq!(bg(&buf, 0, 0), Color::Rgb(want.0, want.1, want.2), "{mode}");
        assert_eq!(bg(&buf, 3, 0), Color::Rgb(200, 100, 50), "{mode}: beside");
    }
}

/// A glyph blends its color with the backdrop's background beneath it
/// (one glyph per cell: the element's glyph shows, in the blended color).
#[test]
fn a_glyph_blends_with_the_backdrop_background() {
    let mut dom = doc(r#"<body><div id="b">ab</div></body>"#);
    let buf = paint(
        &mut dom,
        &format!("{PAGE} #b {{ color: rgb(255, 255, 255); mix-blend-mode: difference }}"),
        4,
        1,
    );
    let c = buf.cell(0, 0).unwrap();
    assert_eq!(c.symbol(), "a");
    assert_eq!(c.fg, Color::Rgb(55, 155, 205));
    assert_eq!(c.bg, Color::Rgb(200, 100, 50));
}

/// §3.2, §5.2: a blending element blends with the content of its parent
/// stacking context — an isolated group — only: under `isolation:
/// isolate` (or any stacking context) a cell the group painted nothing on
/// is transparent, and the element's color shows as it is; one the group
/// painted blends with it.
#[test]
fn isolation_limits_the_backdrop_to_the_group() {
    let css = |iso: &str| {
        format!(
            "{PAGE} #g {{ {iso} }} #in {{ width: 2; background-color: rgb(100, 150, 200); mix-blend-mode: multiply }}"
        )
    };
    let markup = r#"<body><div id="g"><div id="in">x</div></div></body>"#;
    let mut dom = doc(markup);
    let buf = paint(&mut dom, &css(""), 4, 1);
    assert_eq!(bg(&buf, 0, 0), Color::Rgb(78, 59, 39), "no group: the page");
    for iso in [
        "isolation: isolate",
        "position: relative; z-index: 0",
        "opacity: 0.999",
    ] {
        let mut dom = doc(markup);
        let buf = paint(&mut dom, &css(iso), 4, 1);
        // (At 0.999 the group composites to the same 8-bit color.)
        assert_eq!(
            bg(&buf, 0, 0),
            Color::Rgb(100, 150, 200),
            "{iso}: transparent backdrop"
        );
    }
    let mut dom = doc(markup);
    let buf = paint(
        &mut dom,
        &css("isolation: isolate; background-color: rgb(200, 100, 50)"),
        4,
        1,
    );
    assert_eq!(
        bg(&buf, 0, 0),
        Color::Rgb(78, 59, 39),
        "the group's own background"
    );
}

/// §3.2, §5.2: `mix-blend-mode` other than `normal` and `isolation:
/// isolate` make a stacking context; `normal` and `auto` do not.
#[test]
fn blending_and_isolation_make_stacking_contexts() {
    for (css, red) in [
        ("mix-blend-mode: multiply", true),
        ("isolation: isolate", true),
        ("mix-blend-mode: normal", false),
        ("isolation: auto", false),
    ] {
        // A white page: `multiply` over it is the identity.
        let mut dom = doc(r#"<body><div id="t"><div id="k">k</div></div></body>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "body {{ background-color: rgb(255, 255, 255); margin: 0 }}
                 #t {{ {css}; background-color: rgb(0, 0, 255); height: 2 }}
                 #k {{ position: relative; z-index: -1; background-color: rgb(255, 0, 0); width: 1; height: 1 }}"
            ),
            6,
            3,
        );
        assert_eq!(bg(&buf, 0, 0) == RED, red, "{css}");
    }
}

/// §3.4: `background-blend-mode` blends an element's background layers
/// with each other; a cell has one layer, its color, so nothing changes.
#[test]
fn background_blend_mode_is_inert() {
    let mut dom = doc(r#"<div id="b">x</div>"#);
    let buf = paint(
        &mut dom,
        "#b { width: 2; background-color: rgb(1, 2, 3); background-blend-mode: difference }",
        4,
        1,
    );
    assert_eq!(bg(&buf, 0, 0), Color::Rgb(1, 2, 3));
}

/// §3.2: inside an isolated group the backdrop is what the group painted
/// — a nested group's paint included, even where it composited to the
/// color already there. A translucent empty black box paints black on the black
/// page inside `#g`, so `multiply` over it blends to black; with the
/// nested paint lost, the backdrop was empty and the red stayed
/// (C15G-FILTER-COVERAGE).
#[test]
fn a_blend_sees_a_nested_groups_paint_in_its_isolated_group() {
    let mut dom = doc(r#"<body><div id="g"><div id="o"></div><div id="b">x</div></div></body>"#);
    let buf = paint(
        &mut dom,
        "body { background-color: #000; margin: 0 }
         #g { isolation: isolate; position: relative }
         #o { width: 2; height: 1; opacity: .9; background-color: #000 }
         #b { position: absolute; top: 0; left: 0; width: 1;
              background-color: rgb(255, 0, 0); mix-blend-mode: multiply }",
        6,
        2,
    );
    assert_eq!(bg(&buf, 0, 0), Color::Rgb(0, 0, 0));
}
