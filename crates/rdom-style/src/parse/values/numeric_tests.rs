//! Tests for the shared `<length-percentage>` leaf and the component
//! splitter, and for the property grammars built on them. One section
//! per CSS-COMPLETE Phase 2 item.

use super::*;
use crate::parse::token::tokenize;

fn t(src: &str) -> Vec<Token> {
    tokenize(src).unwrap()
}

// ── Splitter ─────────────────────────────────────────────────────────

#[test]
fn components_keep_functions_and_signs_whole() {
    let tokens = t("1 -2 10% calc(50% - (1 + 2)) -5%");
    let parts = components(&tokens).unwrap();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[1], &t("-2")[..]);
    assert_eq!(parts[3], &t("calc(50% - (1 + 2))")[..]);
    assert_eq!(parts[4], &t("-5%")[..]);
    assert!(components(&t("calc(1")).is_none(), "unbalanced");
}

// ── C2-PERCENT ───────────────────────────────────────────────────────

/// CSS Values 4 §5.6: `<length-percentage>` is a length or a
/// percentage; a negative literal is out of range where the property
/// restricts it to `[0,∞]` (§4.1).
#[test]
fn length_percentage_leaves() {
    assert_eq!(
        length_percentage(&t("10%"), Range::NonNegative),
        Some(LengthPercentage::Expr(CalcExpr::Percent(10.0)))
    );
    assert_eq!(
        length_percentage(&t("-10%"), Range::Any),
        Some(LengthPercentage::Expr(CalcExpr::Percent(-10.0)))
    );
    assert_eq!(length_percentage(&t("-10%"), Range::NonNegative), None);
    assert_eq!(
        length_percentage(&t("-3"), Range::Any),
        Some(LengthPercentage::Integer(-3))
    );
    assert_eq!(
        length_percentage(&t("calc(5 / 2)"), Range::NonNegative),
        Some(LengthPercentage::Cells(2.5))
    );
    assert_eq!(length_percentage(&t("auto"), Range::Any), None);
}

/// Every length-bearing property takes the percentage (CSS Box 3 §3.2 /
/// §4.2, Sizing 3 §5.2, Position 3 §3.1) and serializes it as written.
#[test]
fn percent_round_trips_through_every_length_property() {
    use crate::TuiStyle;
    use crate::property_dispatch::{serialize, set};
    for (name, value, expected) in [
        ("padding", "10% 2", "10% 2 10% 2"),
        ("padding-top", "10%", "10%"),
        ("margin", "-10% auto", "-10% auto -10% auto"),
        ("margin-left", "5%", "5%"),
        ("top", "50%", "50%"),
        ("left", "-25%", "-25%"),
        ("min-width", "50%", "50%"),
        ("max-width", "25%", "25%"),
        ("min-height", "calc(25% + 2)", "calc(25% + 2)"),
        ("max-height", "75%", "75%"),
    ] {
        let mut s = TuiStyle::default();
        set(name, value, &mut s).unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
        assert_eq!(serialize(name, &s).as_deref(), Some(expected), "{name}");
    }
    let mut s = TuiStyle::default();
    set("inset", "10% auto -5% calc(50% + 1)", &mut s).unwrap();
    assert_eq!(serialize("bottom", &s).as_deref(), Some("-5%"));
    assert_eq!(serialize("left", &s).as_deref(), Some("calc(50% + 1)"));
    set("opacity", "50%", &mut s).unwrap();
    assert_eq!(s.opacity, Some(crate::Value::Specified(0.5)));
    // `[0,∞]` properties reject a negative literal (Values 4 §4.1).
    for (name, value) in [("padding", "-10%"), ("max-width", "-1%"), ("gap", "-5%")] {
        assert!(set(name, value, &mut s).is_err(), "{name}: {value}");
    }
}

// ── C2-MINMAX ────────────────────────────────────────────────────────

/// CSS Values 4 §10.2: `min( <calc-sum># )`, `max( <calc-sum># )`,
/// `clamp( [<calc-sum> | none], <calc-sum>, [<calc-sum> | none] )`. A
/// constant one folds to cells at parse time; a percent-bearing one
/// stays symbolic and serializes as written.
#[test]
fn comparison_functions_parse_fold_and_serialize() {
    use crate::TuiStyle;
    use crate::property_dispatch::{serialize, set};
    assert_eq!(
        length_percentage(&t("max(3, 7.5)"), Range::NonNegative),
        Some(LengthPercentage::Cells(7.5))
    );
    assert_eq!(
        length_percentage(&t("clamp(1, 5, none)"), Range::NonNegative),
        Some(LengthPercentage::Cells(5.0))
    );
    for (name, value) in [
        ("width", "min(50%, 30)"),
        ("max-width", "clamp(none, 90%, 20)"),
        ("padding-left", "calc(min(10%, 3) * 2)"),
        ("left", "max(-5, 10%)"),
    ] {
        let mut s = TuiStyle::default();
        set(name, value, &mut s).unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
        assert_eq!(serialize(name, &s).as_deref(), Some(value), "{name}");
    }
    for bad in [
        "min()",
        "max(1,)",
        "clamp(1, 2)",
        "clamp(1, none, 3)",
        "min(1 2)",
    ] {
        assert_eq!(length_percentage(&t(bad), Range::Any), None, "{bad}");
    }
}

// ── C2-STEPPED ───────────────────────────────────────────────────────

/// CSS Values 4 §10.3 / §10.7 grammars: `round(<rounding-strategy>?,
/// <calc-sum>, <calc-sum>?)`, `mod()` / `rem()` with two arguments,
/// `abs()` / `sign()` with one. They serialize as written.
#[test]
fn stepped_functions_parse_and_serialize() {
    use crate::TuiStyle;
    use crate::property_dispatch::{serialize, set};
    assert_eq!(
        length_percentage(&t("round(up, 21, 4)"), Range::NonNegative),
        Some(LengthPercentage::Cells(24.0))
    );
    for value in [
        "round(down, 50%, 4)",
        "round(to-zero, 50%)",
        "round(50%, 4)",
        "mod(50%, 7)",
        "rem(50%, 7)",
        "abs(50% - 30)",
        "calc(sign(50% - 10) * 3)",
    ] {
        let mut s = TuiStyle::default();
        set("width", value, &mut s).unwrap_or_else(|e| panic!("{value}: {e:?}"));
        assert_eq!(serialize("width", &s).as_deref(), Some(value));
    }
    for bad in [
        "round(sideways, 1, 2)",
        "round(up)",
        "mod(1)",
        "rem(1, 2, 3)",
        "abs()",
        "sign(1, 2)",
    ] {
        assert_eq!(length_percentage(&t(bad), Range::Any), None, "{bad}");
    }
}
