//! Dispatch tests for `clip-path` and the `mask*` properties (C15-CLIP-PATH;
//! CSS Masking 1 §5.1, §6–§7, CSS Shapes 1 §3.1).

use super::*;
use crate::TuiStyle;

fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// Masking 1 §5.1: `none | <clip-source> | [<basic-shape> ||
/// <geometry-box>]`; Shapes 1 §3.1's `inset()`, `circle()`, `ellipse()`,
/// `polygon()`, `path()` — lengths in cells (pixels are geometry,
/// rejected), percentages of the reference box, positions as two
/// percentages or lengths.
#[test]
fn clip_path_parses() {
    for (css, out) in [
        ("none", "none"),
        ("url(#c)", "url(\"#c\")"),
        ("inset(1)", "inset(1)"),
        ("inset(1 2)", "inset(1 2)"),
        ("inset(1 2 3 4 round 1)", "inset(1 2 3 4 round 1)"),
        ("inset(10% round 1 2)", "inset(10% round 1 2)"),
        ("circle()", "circle()"),
        ("circle(3 at left top)", "circle(3 at 0% 0%)"),
        (
            "circle(farthest-side at 2 50%)",
            "circle(farthest-side at 2 50%)",
        ),
        ("ellipse(4 2)", "ellipse(4 2)"),
        (
            "ellipse(closest-side 50% at center bottom)",
            "ellipse(closest-side 50% at 50% 100%)",
        ),
        (
            "polygon(0 0, 100% 0, 50% 100%)",
            "polygon(0 0, 100% 0, 50% 100%)",
        ),
        (
            "polygon(evenodd, 0 0, 4 0, 0 4)",
            "polygon(evenodd, 0 0, 4 0, 0 4)",
        ),
        ("path('M 0 0 L 4 4')", "path(\"M 0 0 L 4 4\")"),
        ("padding-box", "padding-box"),
        ("circle(2) content-box", "circle(2) content-box"),
        ("margin-box inset(1)", "inset(1) margin-box"),
        ("inset(1) border-box", "inset(1)"),
    ] {
        assert_eq!(round("clip-path", css).as_deref(), Some(out), "{css}");
    }
    for bad in [
        "inset()",
        "inset(1px)",
        "inset(1 2 3 4 5)",
        "circle(-1)",
        "circle(1 2)",
        "ellipse(1)",
        "polygon(0 0, 1)",
        "path(1)",
        "square(1)",
        "padding-box padding-box",
        "none inset(1)",
        "",
    ] {
        assert_eq!(round("clip-path", bad), None, "{bad}");
    }
    assert!(!inherits("clip-path"));
}

/// Masking 1 §6–§7: the mask longhands parse by their grammars and are
/// kept as written; a cell has no alpha to mask by, so they draw nothing.
#[test]
fn mask_properties_parse_and_are_kept() {
    for (name, css) in [
        ("mask-image", "url(m.svg), none"),
        ("mask-mode", "alpha, luminance, match-source"),
        ("mask-repeat", "no-repeat, repeat-x"),
        ("mask-position", "center, 0 0"),
        ("mask-clip", "border-box, no-clip"),
        ("mask-origin", "content-box, fill-box"),
        ("mask-size", "contain, 50% auto"),
        ("mask-composite", "add, exclude"),
        ("mask-type", "alpha"),
        ("mask-border-source", "url(b.svg)"),
        ("mask-border-slice", "30 fill"),
        ("mask-border-width", "1 auto"),
        ("mask-border-outset", "1"),
        ("mask-border-repeat", "round stretch"),
        ("mask-border-mode", "luminance"),
    ] {
        assert!(round(name, css).is_some(), "{name}: {css}");
        assert!(!inherits(name), "{name}");
    }
    assert_eq!(
        round("mask-image", "url(m.svg)").as_deref(),
        Some("url(\"m.svg\")")
    );
    assert_eq!(
        round("mask-type", "luminance").as_deref(),
        Some("luminance")
    );
    for (name, bad) in [
        ("mask-mode", "red"),
        ("mask-composite", "over"),
        ("mask-type", "match-source"),
        ("mask-border-repeat", "a b"),
        ("mask-border-mode", "alpha luminance"),
        ("mask-repeat", "repeat repeat repeat"),
    ] {
        assert_eq!(round(name, bad), None, "{name}: {bad}");
    }
    // The shorthands set their longhands.
    let mut style = TuiStyle::new();
    set(
        "mask",
        "url(m.svg) center / contain no-repeat luminance",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        serialize("mask-image", &style).as_deref(),
        Some("url(\"m.svg\")")
    );
    assert_eq!(serialize("mask-size", &style).as_deref(), Some("contain"));
    assert_eq!(serialize("mask-mode", &style).as_deref(), Some("luminance"));
    set("mask-border", "url(b.svg) 30 / 1 round", &mut style).unwrap();
    assert_eq!(
        serialize("mask-border-source", &style).as_deref(),
        Some("url(\"b.svg\")")
    );
    assert_eq!(
        serialize("mask-border-repeat", &style).as_deref(),
        Some("round")
    );
}

/// CSS 2.1 §11.1.2 (CSS Masking 1 §6.1, deprecated but the web's
/// visually-hidden idiom): `clip: auto | rect(<top>, <right>, <bottom>,
/// <left>)`, each edge a cell length or `auto`, comma-separated or, the
/// legacy form, space-separated; serialized with commas. A pixel length is
/// geometry and rejected (DESIGN) (C15G-LEGACY-CLIP).
#[test]
fn legacy_clip_parses() {
    for (css, out) in [
        ("auto", "auto"),
        ("rect(0, 0, 0, 0)", "rect(0, 0, 0, 0)"),
        ("rect(0 0 0 0)", "rect(0, 0, 0, 0)"),
        ("rect(1, 5, 3, auto)", "rect(1, 5, 3, auto)"),
        ("RECT(auto auto auto auto)", "rect(auto, auto, auto, auto)"),
    ] {
        assert_eq!(round("clip", css).as_deref(), Some(out), "{css}");
    }
    for bad in [
        "rect(0, 0, 0)",
        "rect(1px, 1px, 1px, 1px)",
        "rect(10%, 0, 0, 0)",
        "none",
        "",
    ] {
        assert_eq!(round("clip", bad), None, "{bad}");
    }
}
