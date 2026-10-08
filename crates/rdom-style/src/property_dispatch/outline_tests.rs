//! Dispatch tests for the outline properties (C12-OUTLINE; CSS UI 4 §5).

use super::*;
use crate::layout::{BorderWidth, OutlineColor, OutlineStyle, PaintLength};
use crate::{TuiColor, TuiStyle, Value};

fn spec<T: Clone>(v: &Option<Value<T>>) -> Option<T> {
    match v {
        Some(Value::Specified(x)) => Some(x.clone()),
        _ => None,
    }
}

/// §5.1: `outline: [<outline-color> || <outline-style> || <outline-width>]`
/// — any order, each omitted one reset to its initial value — serialized
/// in canonical order with the initial values left out.
#[test]
fn the_shorthand_sets_and_resets_the_longhands() {
    let mut style = TuiStyle::new();
    set("outline", "red dashed 5px", &mut style).unwrap();
    assert_eq!(spec(&style.ui.outline_style), Some(OutlineStyle::Dashed));
    assert_eq!(
        spec(&style.ui.outline_width),
        Some(BorderWidth::Length(PaintLength::Px(5.0)))
    );
    assert!(matches!(
        spec(&style.ui.outline_color),
        Some(OutlineColor::Color(TuiColor::Literal(_)))
    ));
    assert_eq!(
        serialize("outline", &style).as_deref(),
        Some("red dashed 5px")
    );
    set("outline", "auto", &mut style).unwrap();
    assert_eq!(spec(&style.ui.outline_style), Some(OutlineStyle::Auto));
    assert_eq!(spec(&style.ui.outline_width), Some(BorderWidth::Medium));
    assert_eq!(spec(&style.ui.outline_color), Some(OutlineColor::Auto));
    assert_eq!(serialize("outline", &style).as_deref(), Some("auto"));
    set("outline", "none", &mut style).unwrap();
    assert_eq!(serialize("outline", &style).as_deref(), Some("none"));
    for bad in [
        "solid dashed",
        "hidden",
        "red blue",
        "-1 solid",
        "half-block",
    ] {
        assert_eq!(
            set("outline", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §5.2–§5.4: the longhands. `outline-style` is `auto | <outline-line-style>`
/// (every `<line-style>` but `hidden`); `outline-width` a `<line-width>`;
/// `outline-color` `auto | <color>`; `outline-offset` a `<length>` of any
/// sign, a pixel length allowed (it selects a one-cell offset).
#[test]
fn the_longhands() {
    let mut style = TuiStyle::new();
    set("outline-style", "double", &mut style).unwrap();
    set("outline-width", "thick", &mut style).unwrap();
    set("outline-color", "auto", &mut style).unwrap();
    set("outline-offset", "-2", &mut style).unwrap();
    assert_eq!(
        serialize("outline-style", &style).as_deref(),
        Some("double")
    );
    assert_eq!(serialize("outline-width", &style).as_deref(), Some("thick"));
    assert_eq!(serialize("outline-color", &style).as_deref(), Some("auto"));
    assert_eq!(serialize("outline-offset", &style).as_deref(), Some("-2"));
    set("outline-offset", "3px", &mut style).unwrap();
    assert_eq!(spec(&style.ui.outline_offset), Some(PaintLength::Px(3.0)));
    assert_eq!(serialize("outline-offset", &style).as_deref(), Some("3px"));
    for (name, bad) in [
        ("outline-style", "hidden"),
        ("outline-style", "half-block"),
        ("outline-width", "-1"),
        ("outline-width", "10%"),
        ("outline-color", "nope"),
        ("outline-offset", "auto"),
        ("outline-offset", "10%"),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
}

/// None inherit (§5: "Inherited: no").
#[test]
fn outlines_do_not_inherit() {
    for name in [
        "outline-style",
        "outline-width",
        "outline-color",
        "outline-offset",
    ] {
        assert!(!inherits(name), "{name}");
    }
}
