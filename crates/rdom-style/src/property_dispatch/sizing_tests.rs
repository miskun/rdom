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
            IntrinsicSize::FitContentLimit(std::sync::Arc::new(CalcExpr::Length(20))),
            "fit-content(20)",
        ),
        (
            "Fit-Content(50%)",
            IntrinsicSize::FitContentLimit(std::sync::Arc::new(CalcExpr::Percent(50.0))),
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

// ── C5-MARGIN-TRIM ──────────────────────────────────────────────────

/// CSS Box 4 §3: `margin-trim: none | [block || inline] | [block-start
/// || inline-start || block-end || inline-end]`, ASCII case-insensitive,
/// serialized in the shortest canonical form; not inherited, initial
/// `none`.
#[test]
fn margin_trim_takes_its_grammar() {
    use crate::layout::MarginTrim;
    let t = |bs, is, be, ie| MarginTrim {
        block_start: bs,
        inline_start: is,
        block_end: be,
        inline_end: ie,
    };
    let cases = [
        ("none", t(false, false, false, false), "none"),
        ("block", t(true, false, true, false), "block"),
        ("INLINE", t(false, true, false, true), "inline"),
        ("inline block", t(true, true, true, true), "block inline"),
        ("block-start", t(true, false, false, false), "block-start"),
        (
            "inline-end block-start",
            t(true, false, false, true),
            "block-start inline-end",
        ),
        (
            "block-end block-start",
            t(true, false, true, false),
            "block",
        ),
        (
            "block-start inline-start block-end inline-end",
            t(true, true, true, true),
            "block inline",
        ),
        (
            "inline-start block-end",
            t(false, true, true, false),
            "inline-start block-end",
        ),
    ];
    for (css, want, text) in cases {
        let mut style = TuiStyle::new();
        set("margin-trim", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        assert_eq!(style.margin_trim, Some(Value::Specified(want)), "{css}");
        assert_eq!(
            serialize("margin-trim", &style).as_deref(),
            Some(text),
            "{css}"
        );
    }
    for bad in [
        "",
        "none block",
        "block block",
        "block block-start",
        "inline inline-end",
        "block-start block-start",
        "auto",
        "1",
    ] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("margin-trim", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("margin-trim"));
    assert_eq!(ComputedStyle::initial().margin_trim, MarginTrim::default());
    assert_eq!(TuiStyle::new().margin_trim(t(true, false, false, false)), {
        let mut s = TuiStyle::new();
        set("margin-trim", "block-start", &mut s).unwrap();
        s
    });
}

// ── C5-CONTAIN-SIZE ─────────────────────────────────────────────────

/// CSS Sizing 4 §6.1: `contain-intrinsic-width` / `-height` take `auto?
/// [ none | <length [0,∞]> ]`; the shorthand `contain-intrinsic-size`
/// one or two of them (width, then height — one value sets both). No
/// percentages; serialized in the shortest form.
#[test]
fn contain_intrinsic_size_takes_its_grammar() {
    use crate::calc::CalcExpr;
    use crate::layout::ContainIntrinsicSize as C;
    let len = |n| Some(CalcExpr::Length(n));
    let cases = [
        (
            "none",
            C {
                auto: false,
                length: None,
            },
            "none",
        ),
        (
            "10",
            C {
                auto: false,
                length: len(10),
            },
            "10",
        ),
        (
            "AUTO 4",
            C {
                auto: true,
                length: len(4),
            },
            "auto 4",
        ),
        (
            "auto none",
            C {
                auto: true,
                length: None,
            },
            "auto none",
        ),
    ];
    for (css, want, text) in cases {
        for name in ["contain-intrinsic-width", "contain-intrinsic-height"] {
            let mut style = TuiStyle::new();
            set(name, css, &mut style).unwrap_or_else(|e| panic!("{name}: {css}: {e:?}"));
            assert_eq!(
                serialize(name, &style).as_deref(),
                Some(text),
                "{name}: {css}"
            );
        }
        let mut style = TuiStyle::new();
        set("contain-intrinsic-size", css, &mut style).unwrap();
        assert_eq!(
            style.contain_intrinsic_width,
            Some(Value::Specified(want.clone()))
        );
        assert_eq!(style.contain_intrinsic_height, Some(Value::Specified(want)));
        assert_eq!(
            serialize("contain-intrinsic-size", &style).as_deref(),
            Some(text)
        );
    }
    let mut style = TuiStyle::new();
    set("contain-intrinsic-size", "auto 10 none", &mut style).unwrap();
    assert_eq!(
        serialize("contain-intrinsic-size", &style).as_deref(),
        Some("auto 10 none")
    );
    assert_eq!(
        serialize("contain-intrinsic-height", &style).as_deref(),
        Some("none")
    );
    for bad in [
        "",
        "50%",
        "-1",
        "auto",
        "auto auto 1",
        "1 2 3",
        "none auto",
        "10 auto",
    ] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("contain-intrinsic-size", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("contain-intrinsic-size"));
}

/// CSS Sizing 4 §6.1 with CSS Logical 1 §4: `contain-intrinsic-inline-size`
/// / `-block-size` are the width / height in rdom's horizontal-tb
/// writing mode — one storage, so the later declaration of the pair wins.
#[test]
fn contain_intrinsic_logical_longhands_share_the_physical_storage() {
    let mut style = TuiStyle::new();
    set("contain-intrinsic-inline-size", "7", &mut style).unwrap();
    set("contain-intrinsic-block-size", "auto 3", &mut style).unwrap();
    assert_eq!(
        serialize("contain-intrinsic-width", &style).as_deref(),
        Some("7")
    );
    assert_eq!(
        serialize("contain-intrinsic-height", &style).as_deref(),
        Some("auto 3")
    );
    assert_eq!(
        serialize("contain-intrinsic-block-size", &style).as_deref(),
        Some("auto 3")
    );
    set("contain-intrinsic-width", "2", &mut style).unwrap();
    assert_eq!(
        serialize("contain-intrinsic-inline-size", &style).as_deref(),
        Some("2")
    );
    assert_eq!(
        property_mask("contain-intrinsic-inline-size"),
        property_mask("contain-intrinsic-width")
    );
}
