//! Dispatch tests for `filter` and `backdrop-filter` (C15-FILTER; Filter
//! Effects 1 §5–§6, Filter Effects 2 §3).

use super::*;
use crate::TuiStyle;

fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// Filter Effects 1 §5–§6: `none | <filter-value-list>` — the color-matrix
/// functions with a `<number> | <percentage>` amount (computed to a number;
/// `grayscale`, `sepia`, `invert`, `opacity` clamped to 1; a negative
/// amount invalid; an omitted one the function's default), `hue-rotate()`
/// an angle, `blur()` a length (pixels kept: it draws nothing),
/// `drop-shadow()` a color and two or three lengths, `url()`.
#[test]
fn filter_parses() {
    for name in ["filter", "backdrop-filter"] {
        for (css, out) in [
            ("none", "none"),
            ("grayscale(50%)", "grayscale(0.5)"),
            ("grayscale()", "grayscale(1)"),
            ("sepia(2)", "sepia(1)"),
            ("invert(1) opacity(150%)", "invert(1) opacity(1)"),
            (
                "saturate(250%) brightness(0.5)",
                "saturate(2.5) brightness(0.5)",
            ),
            ("contrast(200%)", "contrast(2)"),
            ("hue-rotate(0.5turn)", "hue-rotate(180deg)"),
            ("hue-rotate(0)", "hue-rotate(0deg)"),
            ("hue-rotate()", "hue-rotate(0deg)"),
            ("blur(2px)", "blur(2px)"),
            ("blur()", "blur(0)"),
            ("drop-shadow(1 2)", "drop-shadow(1 2)"),
            ("drop-shadow(red 1 -1px 3px)", "drop-shadow(1 -1px 3px red)"),
            ("url(#f) Invert(0.25)", "url(\"#f\") invert(0.25)"),
        ] {
            assert_eq!(round(name, css).as_deref(), Some(out), "{name}: {css}");
        }
        for bad in [
            "grayscale(-1)",
            "brightness(red)",
            "hue-rotate(10)",
            "blur(-1px)",
            "blur(50%)",
            "drop-shadow(1)",
            "drop-shadow(1 2 3 4)",
            "drop-shadow(inset 1 2)",
            "frob(1)",
            "none invert(1)",
            "invert(1),",
            "",
        ] {
            assert_eq!(round(name, bad), None, "{name}: {bad}");
        }
        assert!(!inherits(name));
    }
}
