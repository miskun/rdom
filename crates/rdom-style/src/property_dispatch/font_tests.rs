//! Dispatch tests for the CSS Fonts 4 properties (Phase 9): the font
//! longhands and the `font` shorthand.

use super::*;
use crate::layout::{FontFamily, FontSize, FontStyle, FontWeight, LineHeight, SystemFont};
use crate::{TuiStyle, Value};

fn round_trip(name: &str, text: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, text, &mut style).unwrap_or_else(|e| panic!("{name}: {text}: {e:?}"));
    serialize(name, &style)
}

/// §2.2: `font-weight` takes the keywords and a number in `[1, 1000]`.
#[test]
fn font_weight_takes_keywords_and_numbers() {
    for (text, out) in [
        ("normal", "normal"),
        ("BOLD", "bold"),
        ("bolder", "bolder"),
        ("lighter", "lighter"),
        ("1", "1"),
        ("650", "650"),
        ("1000", "1000"),
        ("calc(500 + 100)", "600"),
    ] {
        assert_eq!(
            round_trip("font-weight", text).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["0", "1001", "-100", "heavy", "100 200"] {
        assert_eq!(
            set("font-weight", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("font-weight"));
}

/// §2.4: `font-style: normal | italic | oblique <angle>?`, the angle in
/// `[-90deg, 90deg]`.
#[test]
fn font_style_takes_oblique_with_an_angle() {
    for (text, out) in [
        ("normal", "normal"),
        ("italic", "italic"),
        ("oblique", "oblique"),
        ("oblique 10deg", "oblique 10deg"),
        ("oblique -0.25turn", "oblique -90deg"),
    ] {
        assert_eq!(
            round_trip("font-style", text).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["oblique 91deg", "italic 10deg", "slanted", "oblique 10"] {
        assert_eq!(
            set("font-style", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §2.5, §2.1, §2.3, §6.11: the inert longhands parse and serialize.
#[test]
fn the_inert_longhands_round_trip() {
    for (name, text, out) in [
        ("font-size", "medium", "medium"),
        ("font-size", "x-large", "x-large"),
        ("font-size", "smaller", "smaller"),
        ("font-size", "16px", "16px"),
        ("font-size", "1.5em", "24px"),
        ("font-size", "120%", "120%"),
        (
            "font-family",
            "\"Fira Code\", monospace",
            "\"Fira Code\", monospace",
        ),
        (
            "font-family",
            "Times New Roman, serif",
            "Times New Roman, serif",
        ),
        ("font-stretch", "condensed", "condensed"),
        ("font-stretch", "87.5%", "87.5%"),
        ("font-width", "expanded", "expanded"),
        ("font-variant", "small-caps", "small-caps"),
    ] {
        assert_eq!(
            round_trip(name, text).as_deref(),
            Some(out),
            "{name}: {text}"
        );
    }
    for (name, bad) in [
        ("font-size", "-1px"),
        ("font-size", "huge"),
        ("font-family", "serif,"),
        ("font-family", "default"),
        ("font-stretch", "wide"),
        ("font-variant", "all-small-caps"),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
    for name in ["font-size", "font-family", "font-stretch", "font-variant"] {
        assert!(inherits(name), "{name}");
    }
}

/// §3.7: the `font` shorthand — the optional prefix in any order, the
/// size, `/ line-height`, the family list — sets every longhand (an
/// omitted one initial, `line-height` included); a system font keyword
/// sets the family to it.
#[test]
fn the_font_shorthand_sets_its_longhands() {
    let mut style = TuiStyle::new();
    set(
        "font",
        "small-caps italic 700 12px/2 \"A B\", serif",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        style.font.weight,
        Some(Value::Specified(FontWeight::Number(700.0)))
    );
    assert_eq!(style.font.style, Some(Value::Specified(FontStyle::Italic)));
    assert_eq!(
        style.text.line_height,
        Some(Value::Specified(LineHeight::Number(2.0)))
    );
    assert_eq!(
        serialize("font", &style).as_deref(),
        Some("italic small-caps 700 12px / 2 \"A B\", serif")
    );
    let mut style = TuiStyle::new();
    set("font", "medium serif", &mut style).unwrap();
    assert_eq!(
        style.text.line_height,
        Some(Value::Specified(LineHeight::Normal))
    );
    assert_eq!(style.font.size, Some(Value::Specified(FontSize::Medium)));
    assert_eq!(serialize("font", &style).as_deref(), Some("medium serif"));
    let mut style = TuiStyle::new();
    set("font", "status-bar", &mut style).unwrap();
    assert_eq!(
        style.font.family,
        Some(Value::Specified(FontFamily::System(SystemFont::StatusBar)))
    );
    assert_eq!(serialize("font", &style).as_deref(), Some("status-bar"));
    assert_eq!(
        round_trip("font", "oblique 10deg bold 1em monospace").as_deref(),
        Some("oblique 10deg bold 16px monospace")
    );
    for bad in [
        "bold",
        "12px",
        "bold bold 12px serif",
        "12px / serif",
        "menu serif",
        "bolder 12px serif",
    ] {
        assert_eq!(
            set("font", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("font"));
}
