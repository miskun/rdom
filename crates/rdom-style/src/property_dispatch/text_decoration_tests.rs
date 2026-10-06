//! Dispatch tests for the CSS Text Decoration 3 / 4 properties (Phase 9):
//! the `text-decoration` shorthand and its longhands (§2), the underline
//! placement properties (§4) and `text-decoration-skip-ink` (§3.2).

use super::*;
use crate::layout::{
    PaintLength, TextDecorationLine, TextDecorationStyle, TextDecorationThickness,
};
use crate::{TuiColor, TuiStyle, Value};

fn line(style: &TuiStyle) -> Option<TextDecorationLine> {
    match &style.text_decoration.line {
        Some(Value::Specified(l)) => Some(*l),
        _ => None,
    }
}

/// CSS Text Decoration 4 §2.6: `text-decoration` is the shorthand of its
/// line, thickness, style and color, in any order, an omitted one
/// initial; it serializes shortest, `none` when all are initial.
#[test]
fn text_decoration_is_the_shorthand_of_four_longhands() {
    for (text, out) in [
        ("underline", "underline"),
        ("UNDERLINE overline", "underline overline"),
        ("line-through red wavy", "line-through wavy red"),
        ("dotted underline 2px blue", "underline 2px dotted blue"),
        ("none", "none"),
        ("blink underline", "underline blink"),
        ("currentcolor solid none auto", "none"),
    ] {
        let mut style = TuiStyle::new();
        set("text-decoration", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            serialize("text-decoration", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    let mut style = TuiStyle::new();
    set("text-decoration", "overline dashed", &mut style).unwrap();
    assert_eq!(line(&style), Some(TextDecorationLine::OVERLINE));
    assert_eq!(
        style.text_decoration.style,
        Some(Value::Specified(TextDecorationStyle::Dashed))
    );
    assert_eq!(
        style.text_decoration.color,
        Some(Value::Specified(TuiColor::CurrentColor))
    );
    assert_eq!(
        serialize("text-decoration-line", &style).as_deref(),
        Some("overline")
    );
    for bad in [
        "underline underline",
        "none underline",
        "solid dotted",
        "red blue",
        "squiggle",
        "",
    ] {
        assert_eq!(
            set("text-decoration", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(!inherits("text-decoration"));
    assert!(!inherits("text-decoration-line"));
    assert!(!inherits("text-decoration-color"));
}

/// §2.1 / §2.3 / §2.4 / §2.5: each longhand's grammar; a thickness takes
/// rdom's cells or the pixel units of the decorating properties.
#[test]
fn the_longhands_take_their_grammars() {
    let mut style = TuiStyle::new();
    set("text-decoration-line", "line-through overline", &mut style).unwrap();
    assert_eq!(
        line(&style),
        Some(TextDecorationLine {
            overline: true,
            line_through: true,
            ..TextDecorationLine::NONE
        })
    );
    for (name, ok, bad) in [
        ("text-decoration-line", "blink", "none blink"),
        ("text-decoration-style", "double", "groove"),
        ("text-decoration-color", "#00ff00", "2px"),
        ("text-decoration-thickness", "from-font", "-1px"),
    ] {
        set(name, ok, &mut TuiStyle::new()).unwrap_or_else(|e| panic!("{name}: {ok}: {e:?}"));
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
    let mut style = TuiStyle::new();
    set("text-decoration-thickness", "0.1em", &mut style).unwrap();
    assert_eq!(
        style.text_decoration.thickness,
        Some(Value::Specified(TextDecorationThickness::Length(
            PaintLength::Px(1.6)
        )))
    );
}

/// §4.1 / §4.2 / §3.2: the underline placement properties and
/// `text-decoration-skip-ink` parse and inherit (they are not drawn).
#[test]
fn the_underline_placement_properties_parse_and_inherit() {
    for (name, values, bad) in [
        (
            "text-underline-offset",
            &["auto", "2px", "-1", "10%"][..],
            "under",
        ),
        (
            "text-underline-position",
            &[
                "auto",
                "from-font",
                "under",
                "left",
                "under right",
                "right under",
            ][..],
            "left right",
        ),
        (
            "text-decoration-skip-ink",
            &["auto", "none", "all"][..],
            "some",
        ),
    ] {
        for value in values {
            let mut style = TuiStyle::new();
            set(name, value, &mut style).unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
            assert!(serialize(name, &style).is_some(), "{name}: {value}");
        }
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
        assert!(inherits(name), "{name}");
    }
    let mut style = TuiStyle::new();
    set("text-underline-position", "right under", &mut style).unwrap();
    assert_eq!(
        serialize("text-underline-position", &style).as_deref(),
        Some("under right")
    );
}
