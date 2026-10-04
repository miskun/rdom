//! Dispatch tests for the borders (CSS Backgrounds 3 §4): the `border`
//! and `border-<side>` shorthands, the style / color / width longhands,
//! serialization and `!important` routing.

use super::*;
use crate::layout::{Border, BorderRadius, BorderStyle, BorderWidth, Corners, PaintLength, Sides};
use crate::{Color, TuiColor, TuiStyle, Value};

fn specified<T: Clone>(v: &Option<Value<T>>) -> T {
    match v {
        Some(Value::Specified(x)) => x.clone(),
        other => panic!("not specified: {}", other.is_some()),
    }
}

/// The four per-side styles as a `Border`.
fn styles(style: &TuiStyle) -> Border {
    Border::from_sides(sides(&style.border_style))
}

fn red() -> TuiColor {
    TuiColor::Literal(Color::Rgb(255, 0, 0))
}

/// The per-side specified values of a `Sides` of declarations.
fn sides<T: Clone>(s: &Sides<Option<Value<T>>>) -> Sides<T> {
    Sides::new(
        specified(&s.top),
        specified(&s.right),
        specified(&s.bottom),
        specified(&s.left),
    )
}

// ── `border` (§4.4) ────────────────────────────────────────────────

/// §4.4: `border: 1px solid red` — width, style and color on all four
/// sides. A pixel width is kept as pixels (DIVERGENCES §2).
#[test]
fn border_shorthand_takes_width_style_and_color() {
    let mut style = TuiStyle::new();
    set("border", "1px solid red", &mut style).unwrap();
    assert_eq!(styles(&style), Border::single());
    assert_eq!(sides(&style.border_color), Sides::all(red()));
    assert_eq!(
        sides(&style.border_width),
        Sides::all(BorderWidth::Length(PaintLength::Px(1.0)))
    );
    assert_eq!(
        serialize("border", &style).as_deref(),
        Some("1px solid red")
    );
}

/// §4.4: `<line-width> || <line-style> || <color>` — any order, each
/// at most once.
#[test]
fn border_shorthand_components_in_any_order() {
    let mut style = TuiStyle::new();
    set("border", "red double thick", &mut style).unwrap();
    assert_eq!(styles(&style), Border::ring(BorderStyle::Double));
    assert_eq!(sides(&style.border_width), Sides::all(BorderWidth::Thick));
    assert_eq!(
        serialize("border", &style).as_deref(),
        Some("thick double red")
    );
    for bad in [
        "solid dashed",
        "1 2",
        "red blue",
        "10% solid",
        "-1px solid",
        "solid wiggly",
        "top red",
    ] {
        assert_eq!(
            set("border", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §4.4: the shorthand resets the components it omits — `border:
/// solid` after `border-color: blue` is `currentcolor` again, width
/// `medium`.
#[test]
fn border_shorthand_resets_omitted_components() {
    let mut style = TuiStyle::new();
    set("border-color", "blue", &mut style).unwrap();
    set("border", "solid", &mut style).unwrap();
    assert_eq!(
        sides(&style.border_color),
        Sides::all(TuiColor::CurrentColor)
    );
    assert_eq!(sides(&style.border_width), Sides::all(BorderWidth::Medium));
    assert_eq!(serialize("border", &style).as_deref(), Some("solid"));
    let mut zero = TuiStyle::new();
    set("border", "0", &mut zero).unwrap();
    assert_eq!(styles(&zero), Border::none());
    assert_eq!(serialize("border", &zero).as_deref(), Some("0"));
}

/// Widths: the keywords, cells, the pixel units and `em` (16px).
#[test]
fn border_width_component_lengths() {
    for (value, width) in [
        ("thin solid", BorderWidth::Thin),
        ("2 solid", BorderWidth::Length(PaintLength::Cells(2.0))),
        ("1ch solid", BorderWidth::Length(PaintLength::Cells(1.0))),
        ("0.5em solid", BorderWidth::Length(PaintLength::Px(8.0))),
        ("1in solid", BorderWidth::Length(PaintLength::Px(96.0))),
    ] {
        let mut style = TuiStyle::new();
        set("border", value, &mut style).unwrap_or_else(|e| panic!("{value}: {e:?}"));
        assert_eq!(specified(&style.border_width.left), width, "{value}");
    }
}

/// rdom's keywords keep working: `rounded` (a solid ring that also
/// sets `border-radius: 1`, now beside a width and color too), `single`
/// (= `solid`), `half-block`, and the one-side `top` / `right` /
/// `bottom` / `left` on their own.
#[test]
fn border_shorthand_keeps_rdom_keywords() {
    let mut style = TuiStyle::new();
    set("border", "1px rounded red", &mut style).unwrap();
    assert_eq!(styles(&style), Border::single());
    assert_eq!(
        style.border_radius,
        Corners::all(Some(Value::Specified(BorderRadius::cells(1.0))))
    );
    assert_eq!(
        serialize("border", &style).as_deref(),
        Some("1px solid red")
    );
    assert_eq!(serialize("border-radius", &style).as_deref(), Some("1"));
    set("border", "single", &mut style).unwrap();
    assert_eq!(styles(&style), Border::single());
    set("border", "half-block red", &mut style).unwrap();
    assert_eq!(styles(&style), Border::ring(BorderStyle::HalfBlock));
    set("border", "left", &mut style).unwrap();
    assert_eq!(styles(&style), Border::left());
    assert_eq!(serialize("border", &style).as_deref(), Some("left"));
    // Elsewhere `rounded` is `solid` alone.
    let mut style = TuiStyle::new();
    set("border-style", "rounded", &mut style).unwrap();
    assert_eq!(styles(&style), Border::single());
    assert!(style.border_radius.top_left.is_none());
}

// ── `border-<side>` (§4.4) ─────────────────────────────────────────

/// §4.4: `border-top: 2 double rgb(1, 2, 3)` sets that side's style,
/// width and color, and only that side's.
#[test]
fn border_side_shorthand_sets_one_side() {
    let mut style = TuiStyle::new();
    set("border", "solid red", &mut style).unwrap();
    set("border-top", "2 double rgb(1, 2, 3)", &mut style).unwrap();
    let b = styles(&style);
    assert_eq!(
        (b.top, b.right, b.left),
        (BorderStyle::Double, BorderStyle::Solid, BorderStyle::Solid)
    );
    assert_eq!(
        specified(&style.border_color.top),
        TuiColor::Literal(Color::Rgb(1, 2, 3))
    );
    assert_eq!(specified(&style.border_color.bottom), red());
    assert_eq!(
        specified(&style.border_width.top),
        BorderWidth::Length(PaintLength::Cells(2.0))
    );
    assert_eq!(specified(&style.border_width.right), BorderWidth::Medium);
    assert_eq!(
        serialize("border-top", &style).as_deref(),
        Some("2 double rgb(1, 2, 3)")
    );
    assert_eq!(
        serialize("border-right", &style).as_deref(),
        Some("solid red")
    );
    // The shorthand needs every side to agree.
    assert_eq!(serialize("border", &style), None);
}

/// `!important` on `border` covers every side's color and width;
/// `border-top`'s covers the top's only.
#[test]
fn border_shorthands_own_their_longhands() {
    let all = property_mask("border").unwrap();
    assert!(all.contains(property_mask("border-color").unwrap()));
    let top = property_mask("border-top").unwrap();
    assert!(top.contains(crate::ImportantMask::BORDER_TOP_COLOR));
    assert!(!top.contains(crate::ImportantMask::BORDER_LEFT_COLOR));
    let mut style = TuiStyle::new();
    set("border", "1px solid red", &mut style).unwrap();
    assert!(remove("border", &mut style));
    assert!(style.border_style.top.is_none() && style.border_color.top.is_none());
    assert!(style.border_width.left.is_none());
}

// ── 1–4 values and per-side longhands (§4.1–§4.3) ──────────────────

/// §4.1: `border-color` takes one to four colors, clockwise from the
/// top, and serializes in the shortest form that reads back the same.
#[test]
fn border_color_takes_one_to_four_values() {
    let blue = TuiColor::Literal(Color::Rgb(0, 0, 255));
    let green = TuiColor::Literal(Color::Rgb(0, 128, 0));
    for (value, expected, serialized) in [
        ("red", Sides::all(red()), "red"),
        (
            "red blue",
            Sides::new(red(), blue.clone(), red(), blue.clone()),
            "red blue",
        ),
        (
            "red blue green",
            Sides::new(red(), blue.clone(), green.clone(), blue.clone()),
            "red blue green",
        ),
        (
            "red blue green red",
            Sides::new(red(), blue.clone(), green.clone(), red()),
            "red blue green red",
        ),
        ("red red red red", Sides::all(red()), "red"),
        (
            "red blue red blue",
            Sides::new(red(), blue.clone(), red(), blue.clone()),
            "red blue",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("border-color", value, &mut style).unwrap_or_else(|e| panic!("{value}: {e:?}"));
        assert_eq!(sides(&style.border_color), expected, "{value}");
        assert_eq!(
            serialize("border-color", &style).as_deref(),
            Some(serialized),
            "{value}"
        );
    }
    assert_eq!(
        set("border-color", "red red red red red", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
}

/// §4.2: `border-style` takes one to four styles.
#[test]
fn border_style_takes_one_to_four_values() {
    let mut style = TuiStyle::new();
    set("border-style", "solid none dashed", &mut style).unwrap();
    let b = styles(&style);
    assert_eq!(
        (b.top, b.right, b.bottom, b.left),
        (
            BorderStyle::Solid,
            BorderStyle::None,
            BorderStyle::Dashed,
            BorderStyle::None
        )
    );
    assert_eq!(
        serialize("border-style", &style).as_deref(),
        Some("solid none dashed")
    );
}

/// §4.3: `border-width` takes one to four `<line-width>`s.
#[test]
fn border_width_takes_one_to_four_values() {
    let mut style = TuiStyle::new();
    set("border-width", "thin 2 thick 1px", &mut style).unwrap();
    assert_eq!(
        sides(&style.border_width),
        Sides::new(
            BorderWidth::Thin,
            BorderWidth::Length(PaintLength::Cells(2.0)),
            BorderWidth::Thick,
            BorderWidth::Length(PaintLength::Px(1.0)),
        )
    );
    assert_eq!(
        serialize("border-width", &style).as_deref(),
        Some("thin 2 thick 1px")
    );
    for bad in ["red", "-1", "10%", "1 2 3 4 5"] {
        assert_eq!(
            set("border-width", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §4.1 / §4.3: `border-<side>-color` and `border-<side>-width` set one
/// side; each is its own longhand with its own `!important` bit.
#[test]
fn per_side_color_and_width_longhands() {
    let mut style = TuiStyle::new();
    set("border-color", "red", &mut style).unwrap();
    set("border-left-color", "blue", &mut style).unwrap();
    set("border-bottom-width", "thick", &mut style).unwrap();
    assert_eq!(specified(&style.border_color.top), red());
    assert_eq!(
        specified(&style.border_color.left),
        TuiColor::Literal(Color::Rgb(0, 0, 255))
    );
    assert_eq!(specified(&style.border_width.bottom), BorderWidth::Thick);
    assert!(style.border_width.top.is_none());
    assert_eq!(
        serialize("border-left-color", &style).as_deref(),
        Some("blue")
    );
    assert_eq!(
        serialize("border-bottom-width", &style).as_deref(),
        Some("thick")
    );
    assert_eq!(
        property_mask("border-left-color"),
        Some(crate::ImportantMask::BORDER_LEFT_COLOR)
    );
    assert_eq!(
        property_mask("border-width").unwrap(),
        crate::ImportantMask::BORDER_TOP_WIDTH
            | crate::ImportantMask::BORDER_RIGHT_WIDTH
            | crate::ImportantMask::BORDER_BOTTOM_WIDTH
            | crate::ImportantMask::BORDER_LEFT_WIDTH
    );
    set("border-left-color", "inherit", &mut style).unwrap();
    assert_eq!(style.border_color.left, Some(Value::Inherit));
    assert_eq!(specified(&style.border_color.right), red());
}

// ── `border-radius` (§5) ───────────────────────────────────────────

/// §5.2: `border-radius` — one to four horizontal radii clockwise from
/// the top-left, then optionally `/` and the vertical ones; lengths in
/// cells, pixels or percentages.
#[test]
fn border_radius_shorthand() {
    let px = |p| PaintLength::Px(p);
    let mut style = TuiStyle::new();
    set("border-radius", "4px", &mut style).unwrap();
    assert_eq!(
        style.border_radius.top_left,
        Some(Value::Specified(BorderRadius::circle(px(4.0))))
    );
    assert_eq!(serialize("border-radius", &style).as_deref(), Some("4px"));
    set("border-radius", "1 2 3 4 / 50%", &mut style).unwrap();
    let r = |c: &Option<Value<BorderRadius>>| specified(c);
    assert_eq!(
        r(&style.border_radius.bottom_right).horizontal,
        PaintLength::Cells(3.0)
    );
    assert!(matches!(
        r(&style.border_radius.bottom_left).vertical,
        PaintLength::Calc(_)
    ));
    assert_eq!(
        serialize("border-radius", &style).as_deref(),
        Some("1 2 3 4 / 50%")
    );
    for bad in ["-1", "1 2 3 4 5", "/ 1", "1 /", "red", "1 / 2 / 3"] {
        assert_eq!(
            set("border-radius", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §5.1: a corner's longhand takes one or two radii.
#[test]
fn corner_radius_longhands() {
    let mut style = TuiStyle::new();
    set("border-top-left-radius", "2 50%", &mut style).unwrap();
    assert_eq!(
        serialize("border-top-left-radius", &style).as_deref(),
        Some("2 50%")
    );
    assert!(style.border_radius.top_right.is_none());
    assert_eq!(
        property_mask("border-bottom-left-radius"),
        Some(crate::ImportantMask::BORDER_BOTTOM_LEFT_RADIUS)
    );
    assert_eq!(
        set("border-top-left-radius", "1 2 3", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
}

/// §4.2: each side's style is its own longhand: `border-top-style:
/// inherit` leaves the other sides.
#[test]
fn per_side_style_longhands_are_independent() {
    let mut style = TuiStyle::new();
    set("border", "solid", &mut style).unwrap();
    set("border-top-style", "inherit", &mut style).unwrap();
    assert_eq!(style.border_style.top, Some(Value::Inherit));
    assert_eq!(
        style.border_style.left,
        Some(Value::Specified(BorderStyle::Solid))
    );
    assert_eq!(
        property_mask("border-top"),
        Some(
            crate::ImportantMask::BORDER_TOP_STYLE
                | crate::ImportantMask::BORDER_TOP_COLOR
                | crate::ImportantMask::BORDER_TOP_WIDTH
        )
    );
}
