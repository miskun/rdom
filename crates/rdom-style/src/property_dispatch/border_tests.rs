//! Dispatch tests for the borders (CSS Backgrounds 3 §4): the `border`
//! and `border-<side>` shorthands, the style / color / width longhands,
//! serialization and `!important` routing.

use super::*;
use crate::layout::{Border, BorderStyle, BorderWidth, CornerStyle, PaintLength, Sides};
use crate::{Color, TuiColor, TuiStyle, Value};

fn specified<T: Clone>(v: &Option<Value<T>>) -> T {
    match v {
        Some(Value::Specified(x)) => x.clone(),
        other => panic!("not specified: {}", other.is_some()),
    }
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
    assert_eq!(specified(&style.border), Border::single());
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
    assert_eq!(specified(&style.border), Border::ring(BorderStyle::Double));
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
    assert_eq!(specified(&zero.border), Border::none());
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

/// rdom's keywords keep working: `rounded` (a solid ring with rounded
/// corners, now also beside a width and color), `single` (= `solid`),
/// `half-block`, and the one-side `top` / `right` / `bottom` / `left`
/// on their own.
#[test]
fn border_shorthand_keeps_rdom_keywords() {
    let mut style = TuiStyle::new();
    set("border", "rounded", &mut style).unwrap();
    assert_eq!(specified(&style.border), Border::rounded());
    assert_eq!(serialize("border", &style).as_deref(), Some("rounded"));
    set("border", "1px rounded red", &mut style).unwrap();
    assert_eq!(specified(&style.border).corner_style, CornerStyle::Rounded);
    assert_eq!(
        serialize("border", &style).as_deref(),
        Some("1px rounded red")
    );
    set("border", "single", &mut style).unwrap();
    assert_eq!(specified(&style.border), Border::single());
    set("border", "half-block red", &mut style).unwrap();
    assert_eq!(
        specified(&style.border),
        Border::ring(BorderStyle::HalfBlock)
    );
    set("border", "left", &mut style).unwrap();
    assert_eq!(specified(&style.border), Border::left());
    assert_eq!(serialize("border", &style).as_deref(), Some("left"));
}

/// rdom's `rounded` on `border-style` is the same solid ring with
/// rounded corners as on `border`; another style there squares them —
/// `border-style` sets the whole ring — while the per-side style
/// longhands keep the corners.
#[test]
fn border_style_rounded_rounds_the_ring() {
    let mut style = TuiStyle::new();
    set("border-style", "rounded", &mut style).unwrap();
    assert_eq!(specified(&style.border), Border::rounded());
    assert_eq!(
        serialize("border-style", &style).as_deref(),
        Some("rounded")
    );
    set("border-top-style", "double", &mut style).unwrap();
    assert_eq!(specified(&style.border).corner_style, CornerStyle::Rounded);
    set("border-style", "solid", &mut style).unwrap();
    assert_eq!(specified(&style.border), Border::single());
}

// ── `border-<side>` (§4.4) ─────────────────────────────────────────

/// §4.4: `border-top: 2 double rgb(1, 2, 3)` sets that side's style,
/// width and color, and only that side's.
#[test]
fn border_side_shorthand_sets_one_side() {
    let mut style = TuiStyle::new();
    set("border", "solid red", &mut style).unwrap();
    set("border-top", "2 double rgb(1, 2, 3)", &mut style).unwrap();
    let b = specified(&style.border);
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
    assert!(style.border.is_none() && style.border_color.top.is_none());
    assert!(style.border_width.left.is_none());
}
