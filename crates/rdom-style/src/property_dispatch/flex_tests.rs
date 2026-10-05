//! Dispatch tests for the flexbox properties of Phase 6 (C6-ORDER, …).

use super::*;
use crate::{ImportantMask, TuiStyle, Value};

/// CSS Flexbox §5.4: `order: <integer>`, initial 0, not inherited; a
/// math function rounds to an integer (CSS Values 4 §10.9) and a value
/// past `i32` clamps to it (§5.1); a non-integer is invalid.
#[test]
fn order_takes_an_integer() {
    for (css, n, out) in [
        ("0", 0, "0"),
        ("-3", -3, "-3"),
        ("7", 7, "7"),
        ("calc(2 * 3)", 6, "6"),
        ("99999999999", i32::MAX, "2147483647"),
    ] {
        let mut style = TuiStyle::new();
        set("order", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        assert_eq!(style.order, Some(Value::Specified(n)), "{css}");
        assert_eq!(serialize("order", &style).as_deref(), Some(out), "{css}");
    }
    for bad in ["1.5", "auto", "1px", "", "1 2"] {
        assert_eq!(
            set("order", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("order"));
    assert_eq!(property_mask("order"), Some(ImportantMask::ORDER));
}

/// CSS Flexbox §5.1: `flex-direction: row | row-reverse | column |
/// column-reverse` — the axis and whether main-start and main-end swap,
/// both owned by the property.
#[test]
fn flex_direction_takes_the_reverse_keywords() {
    use crate::layout::Direction;
    for (css, axis, reverse) in [
        ("row", Direction::Row, false),
        ("row-reverse", Direction::Row, true),
        ("column", Direction::Column, false),
        ("COLUMN-REVERSE", Direction::Column, true),
    ] {
        let mut style = TuiStyle::new();
        set("flex-direction", css, &mut style).unwrap();
        assert_eq!(style.direction, Some(Value::Specified(axis)), "{css}");
        assert_eq!(style.flex_reverse, Some(Value::Specified(reverse)), "{css}");
        assert_eq!(
            serialize("flex-direction", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    assert_eq!(
        property_mask("flex-direction"),
        Some(ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE)
    );
    let mut style = TuiStyle::new();
    set("flex-direction", "row-reverse", &mut style).unwrap();
    set("flex-direction", "column", &mut style).unwrap();
    assert_eq!(style.flex_reverse, Some(Value::Specified(false)));
}

/// CSS Flexbox §7.3.1 / §7.3.3: `flex-grow: <number [0,∞]>` (initial
/// 0) and `flex-basis: content | <'width'>` — `auto`, `content`, a
/// `<length-percentage [0,∞]>` and the intrinsic size keywords (CSS
/// Sizing 3 §3.1). The `flex` shorthand serializes from the three
/// longhands, and owns exactly them.
#[test]
fn flex_grow_and_flex_basis_longhands() {
    use crate::calc::CalcExpr;
    use crate::layout::{FlexBasis, IntrinsicSize};
    for (css, n) in [("0", 0.0), ("1.5", 1.5), ("calc(2 * 2)", 4.0)] {
        let mut style = TuiStyle::new();
        set("flex-grow", css, &mut style).unwrap();
        assert_eq!(style.flex_grow, Some(Value::Specified(n)), "{css}");
    }
    for bad in ["-1", "auto", "1px"] {
        assert_eq!(
            set("flex-grow", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    for (css, basis, out) in [
        ("auto", FlexBasis::Auto, "auto"),
        ("content", FlexBasis::Content, "content"),
        ("4", FlexBasis::Cells(4), "4"),
        (
            "50%",
            FlexBasis::Calc(Box::new(CalcExpr::Percent(50.0))),
            "50%",
        ),
        (
            "min-content",
            FlexBasis::Intrinsic(IntrinsicSize::MinContent),
            "min-content",
        ),
        (
            "max-content",
            FlexBasis::Intrinsic(IntrinsicSize::MaxContent),
            "max-content",
        ),
        (
            "fit-content",
            FlexBasis::Intrinsic(IntrinsicSize::FitContent),
            "fit-content",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("flex-basis", css, &mut style).unwrap();
        assert_eq!(style.flex_basis, Some(Value::Specified(basis)), "{css}");
        assert_eq!(
            serialize("flex-basis", &style).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in ["-1", "none", "1 2"] {
        assert_eq!(
            set("flex-basis", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    let mut style = TuiStyle::new();
    set("flex-grow", "2", &mut style).unwrap();
    set("flex-shrink", "0", &mut style).unwrap();
    set("flex-basis", "30%", &mut style).unwrap();
    assert_eq!(serialize("flex", &style).as_deref(), Some("2 0 30%"));
    assert_eq!(
        property_mask("flex"),
        Some(ImportantMask::FLEX_GROW | ImportantMask::FLEX_SHRINK | ImportantMask::FLEX_BASIS)
    );
    assert!(!inherits("flex-grow") && !inherits("flex-basis"));
}

/// CSS Box Alignment 3 §8.1 / §8.3: `row-gap` and `column-gap` take
/// `normal | <length-percentage [0,∞]>` (initial `normal`); `gap` is
/// `<'row-gap'> <'column-gap'>?`, one value for both.
#[test]
fn row_gap_column_gap_and_the_gap_shorthand() {
    use crate::calc::CalcExpr;
    use crate::layout::GapValue;
    let spec = |g| Some(Value::Specified(g));
    let mut style = TuiStyle::new();
    set("row-gap", "2", &mut style).unwrap();
    set("column-gap", "normal", &mut style).unwrap();
    assert_eq!(style.row_gap, spec(GapValue::Cells(2)));
    assert_eq!(style.column_gap, spec(GapValue::Normal));
    assert_eq!(serialize("column-gap", &style).as_deref(), Some("normal"));
    assert_eq!(serialize("gap", &style).as_deref(), Some("2 normal"));
    for (css, row, column, out) in [
        ("1", GapValue::Cells(1), GapValue::Cells(1), "1"),
        ("1 3", GapValue::Cells(1), GapValue::Cells(3), "1 3"),
        (
            "normal 10%",
            GapValue::Normal,
            GapValue::Calc(Box::new(CalcExpr::Percent(10.0))),
            "normal 10%",
        ),
        ("2 2", GapValue::Cells(2), GapValue::Cells(2), "2"),
    ] {
        let mut style = TuiStyle::new();
        set("gap", css, &mut style).unwrap();
        assert_eq!(
            (style.row_gap.clone(), style.column_gap.clone()),
            (spec(row), spec(column)),
            "{css}"
        );
        assert_eq!(serialize("gap", &style).as_deref(), Some(out), "{css}");
    }
    for bad in ["1 2 3", "-1", "auto", "1 -2"] {
        assert_eq!(
            set("gap", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert_eq!(
        property_mask("gap"),
        Some(ImportantMask::ROW_GAP | ImportantMask::COLUMN_GAP)
    );
    assert_eq!(GapValue::Normal.resolve(7), 0);
}

/// CSS Flexbox §5.2: `flex-wrap: nowrap | wrap | wrap-reverse`, initial
/// `nowrap`, not inherited.
#[test]
fn flex_wrap_takes_its_three_keywords() {
    use crate::layout::FlexWrap;
    for (css, kw) in [
        ("nowrap", FlexWrap::NoWrap),
        ("wrap", FlexWrap::Wrap),
        ("WRAP-REVERSE", FlexWrap::WrapReverse),
    ] {
        let mut style = TuiStyle::new();
        set("flex-wrap", css, &mut style).unwrap();
        assert_eq!(style.flex_wrap, Some(Value::Specified(kw)), "{css}");
        assert_eq!(
            serialize("flex-wrap", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["", "auto", "wrap wrap", "reverse"] {
        assert_eq!(
            set("flex-wrap", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("flex-wrap"));
    assert_eq!(property_mask("flex-wrap"), Some(ImportantMask::FLEX_WRAP));
    assert_eq!(crate::ComputedStyle::initial().flex_wrap, FlexWrap::NoWrap);
}

/// CSS Flexbox §5.3: `flex-flow: <'flex-direction'> || <'flex-wrap'>` —
/// either order, an omitted component its initial value; it owns the
/// three fields of its two longhands and serializes in the shortest form
/// (CSSOM §6.7.2).
#[test]
fn flex_flow_sets_direction_and_wrap() {
    use crate::layout::{Direction, FlexWrap};
    for (css, axis, reverse, wrap, out) in [
        ("row wrap", Direction::Row, false, FlexWrap::Wrap, "wrap"),
        (
            "wrap-reverse column",
            Direction::Column,
            false,
            FlexWrap::WrapReverse,
            "column wrap-reverse",
        ),
        (
            "column-reverse",
            Direction::Column,
            true,
            FlexWrap::NoWrap,
            "column-reverse",
        ),
        ("wrap", Direction::Row, false, FlexWrap::Wrap, "wrap"),
        ("row nowrap", Direction::Row, false, FlexWrap::NoWrap, "row"),
    ] {
        let mut style = TuiStyle::new();
        set("flex-direction", "column-reverse", &mut style).unwrap();
        set("flex-wrap", "wrap", &mut style).unwrap();
        set("flex-flow", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        assert_eq!(style.direction, Some(Value::Specified(axis)), "{css}");
        assert_eq!(style.flex_reverse, Some(Value::Specified(reverse)), "{css}");
        assert_eq!(style.flex_wrap, Some(Value::Specified(wrap)), "{css}");
        assert_eq!(
            serialize("flex-flow", &style).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in ["", "row column", "wrap nowrap", "row wrap row", "auto"] {
        assert_eq!(
            set("flex-flow", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert_eq!(
        property_mask("flex-flow"),
        Some(
            ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE | ImportantMask::FLEX_WRAP
        )
    );
    // Only the longhands set: no shorthand to serialize.
    let mut style = TuiStyle::new();
    set("flex-wrap", "wrap", &mut style).unwrap();
    assert_eq!(serialize("flex-flow", &style), None);
}
