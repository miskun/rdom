//! Dispatch tests for the transform properties (C15-TRANSLATE): `translate`
//! (CSS Transforms 2 §6.1), `transform` (CSS Transforms 1 §5, Transforms 2
//! §7), `transform-origin` (Transforms 1 §6), `transform-box` (§7),
//! `rotate` and `scale` (Transforms 2 §6.2–§6.3).

use super::*;
use crate::TuiStyle;

/// `value` set on `name` and serialized back; `None` when the declaration
/// is invalid.
fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// Transforms 2 §6.1: `none | <length-percentage> [<length-percentage>
/// <length>?]?` — whole cells or percentages on x and y; a pixel length
/// is geometry and is rejected (DESIGN, "Pixel lengths select, cells
/// measure"); the z offset has no meaning on a grid and takes any length.
/// Serialized with the trailing zero components dropped.
#[test]
fn translate_parses() {
    assert_eq!(round("translate", "none").as_deref(), Some("none"));
    assert_eq!(round("translate", "3").as_deref(), Some("3"));
    assert_eq!(round("translate", "-3 2").as_deref(), Some("-3 2"));
    assert_eq!(round("translate", "50% -1").as_deref(), Some("50% -1"));
    assert_eq!(round("translate", "1 0").as_deref(), Some("1"));
    assert_eq!(round("translate", "1 2 0").as_deref(), Some("1 2"));
    assert_eq!(round("translate", "1 2 5px").as_deref(), Some("1 2 5px"));
    assert_eq!(
        round("translate", "2ch calc(50% + 1)").as_deref(),
        Some("2 calc(50% + 1)")
    );
    for bad in [
        "10px", "1 2px", "auto", "1 2 3%", "red", "1 2 3 4", "none 1", "",
    ] {
        assert_eq!(round("translate", bad), None, "{bad}");
    }
    assert!(!inherits("translate"));
}

/// Transforms 1 §5 / Transforms 2 §7: `none | <transform-list>`. The
/// translate functions take `<length-percentage>`s (no pixels); every
/// other function parses by its grammar and is kept as written, inert.
#[test]
fn transform_parses() {
    for (css, out) in [
        ("none", "none"),
        ("translate(1, 2)", "translate(1, 2)"),
        ("translate(1)", "translate(1)"),
        ("TranslateX(50%)", "translateX(50%)"),
        ("translateY(-1)", "translateY(-1)"),
        ("translate3d(1, 2, 0)", "translate3d(1, 2, 0)"),
        ("translateZ(0)", "translateZ(0)"),
        (
            "translateX(1) rotate(45deg) scale(2)",
            "translateX(1) rotate(45deg) scale(2)",
        ),
        ("scale(1.5, 50%)", "scale(1.5, 50%)"),
        ("skew(10deg, 0)", "skew(10deg, 0)"),
        ("skewX(-0.5turn)", "skewX(-0.5turn)"),
        ("matrix(1, 0, 0, 1, 0, 0)", "matrix(1, 0, 0, 1, 0, 0)"),
        (
            "perspective(500px) rotateY(10deg)",
            "perspective(500px) rotateY(10deg)",
        ),
        ("rotate3d(0, 0, 1, 90deg)", "rotate3d(0, 0, 1, 90deg)"),
        ("scale3d(1, 2, 3)", "scale3d(1, 2, 3)"),
    ] {
        assert_eq!(round("transform", css).as_deref(), Some(out), "{css}");
    }
    for bad in [
        "translate(10px)",
        "translate(1 2)",
        "translate()",
        "translate(1, 2, 3)",
        "translateZ(5%)",
        "rotate(red)",
        "rotate(45)",
        "matrix(1, 2)",
        "scale()",
        "skew(1deg, 2deg, 3deg)",
        "frob(1)",
        "none translate(1)",
        "translate(1),",
        "",
    ] {
        assert_eq!(round("transform", bad), None, "{bad}");
    }
    assert!(!inherits("transform"));
}

/// Transforms 2 §6.2: `none | <angle> | [x | y | z | <number>{3}] &&
/// <angle>`; §6.3: `none | [<number> | <percentage>]{1,3}` (a percentage
/// computes to a number). Parsed and kept; a cell grid rotates and
/// scales nothing.
#[test]
fn rotate_and_scale_parse() {
    for (css, out) in [
        ("none", "none"),
        ("45deg", "45deg"),
        ("x 90deg", "x 90deg"),
        ("90deg y", "y 90deg"),
        ("0 0 1 0.5turn", "z 180deg"),
        ("1 2 3 10deg", "1 2 3 10deg"),
    ] {
        assert_eq!(round("rotate", css).as_deref(), Some(out), "{css}");
    }
    for bad in ["45", "x y 1deg", "1 2 1deg", "red"] {
        assert_eq!(round("rotate", bad), None, "{bad}");
    }
    for (css, out) in [
        ("none", "none"),
        ("2", "2"),
        ("2 2", "2"),
        ("2 3", "2 3"),
        ("50%", "0.5"),
        ("1 2 1", "1 2"),
        ("1 2 3", "1 2 3"),
    ] {
        assert_eq!(round("scale", css).as_deref(), Some(out), "{css}");
    }
    for bad in ["1 2 3 4", "2px", "red"] {
        assert_eq!(round("scale", bad), None, "{bad}");
    }
    assert!(!inherits("rotate"));
    assert!(!inherits("scale"));
}

/// Transforms 1 §6: `transform-origin` — one to three positions (a length
/// may be pixels: the origin moves nothing in a grid, it is kept), a
/// keyword computing to its percentage;
/// §7: `transform-box`'s five keywords.
#[test]
fn transform_origin_and_box_parse() {
    for (css, out) in [
        ("left top", "0% 0%"),
        ("50% 50%", "50% 50%"),
        ("10px 2 0", "10px 2"),
        ("center", "50% 50%"),
        ("top", "50% 0%"),
        ("bottom right 1px", "100% 100% 1px"),
    ] {
        assert_eq!(
            round("transform-origin", css).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in ["red", "left right", "1 2 3 4", "top top", ""] {
        assert_eq!(round("transform-origin", bad), None, "{bad}");
    }
    for k in [
        "content-box",
        "border-box",
        "fill-box",
        "stroke-box",
        "view-box",
    ] {
        assert_eq!(round("transform-box", k).as_deref(), Some(k));
    }
    assert_eq!(round("transform-box", "padding-box"), None);
    assert!(!inherits("transform-origin"));
    assert!(!inherits("transform-box"));
}
