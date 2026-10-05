//! Dispatch tests for box sizing (CSS-COMPLETE Phase 5): `box-sizing`.

use super::*;
use crate::layout::BoxSizing;
use crate::{ComputedStyle, TuiStyle, Value};

/// CSS UI 3 §3.1 (now CSS Sizing 3, "Box Edges for Sizing"):
/// `box-sizing: content-box | border-box`, ASCII case-insensitive,
/// serialized as the keyword; anything else is invalid.
#[test]
fn box_sizing_takes_its_two_keywords() {
    for (css, kw) in [
        ("content-box", BoxSizing::ContentBox),
        ("BORDER-BOX", BoxSizing::BorderBox),
    ] {
        let mut style = TuiStyle::new();
        set("box-sizing", css, &mut style).unwrap();
        assert_eq!(style.box_sizing, Some(Value::Specified(kw)));
        assert_eq!(
            serialize("box-sizing", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["padding-box", "auto", "border-box content-box", ""] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("box-sizing", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS UI 3 §3.1: initial `content-box`, not inherited — so `unset`
/// is `initial`.
#[test]
fn box_sizing_is_content_box_initially_and_not_inherited() {
    assert_eq!(ComputedStyle::initial().box_sizing, BoxSizing::ContentBox);
    assert_eq!(BoxSizing::default(), BoxSizing::ContentBox);
    assert!(!inherits("box-sizing"));
    let mut style = TuiStyle::new();
    set("box-sizing", "unset", &mut style).unwrap();
    assert_eq!(style.box_sizing, Some(Value::Initial));
}

/// The builder setter writes the same field as the declaration, and
/// `!important` routes through the field's own bit.
#[test]
fn box_sizing_builder_matches_the_declaration() {
    let mut declared = TuiStyle::new();
    set("box-sizing", "border-box", &mut declared).unwrap();
    assert_eq!(TuiStyle::new().box_sizing(BoxSizing::BorderBox), declared);
    let important = TuiStyle::new().box_sizing_important(BoxSizing::BorderBox);
    assert!(
        important
            .important
            .contains(crate::ImportantMask::BOX_SIZING)
    );
    assert_eq!(
        property_mask("box-sizing"),
        Some(crate::ImportantMask::BOX_SIZING)
    );
}

// ── C5-INTRINSIC ────────────────────────────────────────────────────

/// CSS Sizing 3 §3.1 / §3.2 / §3.3: `width` / `height`, `min-*` and
/// `max-*` take `min-content | max-content | fit-content |
/// fit-content(<length-percentage [0,∞]>)`, ASCII case-insensitive,
/// serialized in their canonical form.
#[test]
fn sizes_take_the_intrinsic_keywords() {
    use crate::calc::CalcExpr;
    use crate::layout::{IntrinsicSize, MaxSize, MinSize, Size};
    let cases = [
        ("min-content", IntrinsicSize::MinContent, "min-content"),
        ("MAX-CONTENT", IntrinsicSize::MaxContent, "max-content"),
        ("fit-content", IntrinsicSize::FitContent, "fit-content"),
        (
            "fit-content(20)",
            IntrinsicSize::FitContentLimit(Box::new(CalcExpr::Length(20))),
            "fit-content(20)",
        ),
        (
            "Fit-Content(50%)",
            IntrinsicSize::FitContentLimit(Box::new(CalcExpr::Percent(50.0))),
            "fit-content(50%)",
        ),
    ];
    for (css, kw, text) in cases {
        let mut style = TuiStyle::new();
        for name in [
            "width",
            "height",
            "min-width",
            "min-height",
            "max-width",
            "max-height",
        ] {
            set(name, css, &mut style).unwrap_or_else(|e| panic!("{name}: {css}: {e:?}"));
            assert_eq!(
                serialize(name, &style).as_deref(),
                Some(text),
                "{name}: {css}"
            );
        }
        assert_eq!(
            style.width,
            Some(Value::Specified(Size::Intrinsic(kw.clone())))
        );
        assert_eq!(
            style.min_height,
            Some(Value::Specified(MinSize::Intrinsic(kw.clone())))
        );
        assert_eq!(
            style.max_width,
            Some(Value::Specified(MaxSize::Intrinsic(kw)))
        );
    }
    // A math function inside keeps its form.
    let mut style = TuiStyle::new();
    set("width", "fit-content(calc(50% - 2))", &mut style).unwrap();
    assert_eq!(
        serialize("width", &style).as_deref(),
        Some("fit-content(calc(50% - 2))")
    );
}

/// CSS Sizing 3 §3.1: `fit-content()` takes one non-negative
/// `<length-percentage>`; anything else is invalid.
#[test]
fn fit_content_rejects_bad_arguments() {
    for bad in [
        "fit-content()",
        "fit-content(-1)",
        "fit-content(auto)",
        "fit-content(1 2)",
        "fit-content(min-content)",
        "min-content max-content",
    ] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("width", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}
