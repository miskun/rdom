//! C2G-CALC-SEMANTICS: the math-function rules of CSS Values 4 §10.9
//! that the first Phase 2 pass approximated — IEEE division, the
//! symmetric range of an infinite result, the type of a percentage,
//! no parse-time folding of what needs a basis, math in `<integer>`
//! properties and registered syntaxes, finite angles.

use super::{CalcExpr, CalcKind, ResolveCtx};
use crate::layout::ZIndex;
use crate::parse::token::tokenize;
use crate::parse::values::{parse_angle, parse_calc};
use crate::property_dispatch::{DispatchError, set};
use crate::{PropertySyntax, TuiStyle, Value};

fn calc(src: &str) -> Option<CalcExpr> {
    parse_calc(&tokenize(src).unwrap())
}

fn at(src: &str, basis: i32) -> i32 {
    calc(src)
        .unwrap_or_else(|| panic!("{src} parses"))
        .resolve(&ResolveCtx::new(basis))
}

fn style(name: &str, value: &str) -> Result<TuiStyle, DispatchError> {
    let mut s = TuiStyle::new();
    set(name, value, &mut s).map(|()| s)
}

/// §10.9 "Type checking": division by zero follows IEEE-754 — `1/0` is
/// +∞, `-1/0` −∞, `0/0` NaN — whether the zero is a literal or computed;
/// inside a math function the infinity is a value like any other
/// (`min(1/0, 5)` is 5), and at the top level ∞ clamps to the range and
/// NaN is 0.
#[test]
fn division_by_zero_is_ieee() {
    assert_eq!(at("calc(1 / 0)", 0), i32::MAX);
    assert_eq!(at("calc(-1 / 0)", 0), -i32::MAX);
    assert_eq!(at("calc(0 / 0)", 0), 0);
    assert_eq!(at("calc(min(1 / 0, 5))", 0), 5);
    assert_eq!(at("calc(max(-1 / 0, 3))", 0), 3);
    // A computed zero: sign(50% - 20) is 0 against a basis of 40.
    assert_eq!(at("calc(10 / sign(50% - 20))", 40), i32::MAX);
    assert_eq!(at("calc(min(10 / sign(50% - 20), 7))", 40), 7);
    assert!(style("width", "calc(10 / 0)").is_ok());
}

/// The range an infinite top-level result clamps to is symmetric, so a
/// consumer can negate it (`right` points inward, `-cells`) without
/// overflowing: `calc(-infinity)` is `-i32::MAX`, not `i32::MIN`.
#[test]
fn infinite_results_clamp_symmetrically() {
    assert_eq!(at("calc(-infinity)", 0), -i32::MAX);
    assert_eq!(at("calc(infinity)", 0), i32::MAX);
    assert_eq!(crate::absolute::cells_i32(f64::NEG_INFINITY), -i32::MAX);
    assert_eq!(crate::absolute::cells_i32(f64::INFINITY), i32::MAX);
}

/// §10.9: a percentage has its own type, «percent». Where the property
/// resolves percentages against a length it joins a length; in a
/// property whose percentages are numbers (`opacity`, CSS Color 4
/// §11.1: `<number> | <percentage>`) it is a number, so `calc(50%)` and
/// `min(1, 50%)` are opacities of 0.5.
#[test]
fn percentages_have_their_own_type() {
    assert_eq!(calc("calc(50%)").unwrap().kind(), Some(CalcKind::Percent));
    assert_eq!(
        calc("calc(50% * 2)").unwrap().kind(),
        Some(CalcKind::Percent)
    );
    assert_eq!(
        calc("calc(50% + 2)").unwrap().kind(),
        Some(CalcKind::Length)
    );
    assert_eq!(calc("calc(50% * 50%)"), None, "percent × percent");
    assert_eq!(calc("calc(2 / 50%)"), None, "division by a percentage");
    assert_eq!(calc("sin(50%)"), None);
    assert!(CalcKind::Percent.is_length(), "a <length-percentage>");
    let opacity = |v: &str| style("opacity", v).ok().and_then(|s| s.opacity);
    assert_eq!(opacity("calc(50%)"), Some(Value::Specified(0.5)));
    assert_eq!(opacity("min(1, 50%)"), Some(Value::Specified(0.5)));
    assert_eq!(opacity("calc(50% + 0.25)"), Some(Value::Specified(0.75)));
    assert_eq!(opacity("calc(50% * 50%)"), Some(Value::Specified(0.25)));
}

/// A `<number>` or `<angle>` value is resolved when parsed, so what it
/// needs a basis for must not be folded against zero: a percentage in a
/// property that takes none (`flex-shrink`, an angle) is invalid by type
/// (§10.9), and a viewport unit (`sign(10vw - 40)`, a `<number>` per the
/// type rules) is rejected — rdom resolves numbers at parse time
/// (DIVERGENCES).
#[test]
fn number_and_angle_math_never_folds_a_basis_at_parse_time() {
    assert_eq!(
        style("flex-shrink", "sign(50% - 10)").err(),
        Some(DispatchError::InvalidValue)
    );
    assert_eq!(
        style("flex-shrink", "calc(50%)").err(),
        Some(DispatchError::InvalidValue)
    );
    assert_eq!(
        style("opacity", "sign(10vw - 40)").err(),
        Some(DispatchError::InvalidValue)
    );
    assert!(style("flex-shrink", "sign(3 - 1)").is_ok());
    let angle = |v: &str| parse_angle(&tokenize(v).unwrap());
    assert_eq!(angle("atan2(50%, 10)"), None);
    assert_eq!(angle("atan2(10vw, 10)"), None);
    assert_eq!(angle("atan2(1, 1)"), Some(45.0));
}

/// §10.9: where the context needs an `<integer>`, a math function's
/// result rounds to the nearest integer (a half toward +∞), and an
/// infinite one clamps to the property's range.
#[test]
fn integer_properties_take_math_functions() {
    let z = |v: &str| style("z-index", v).ok().and_then(|s| s.z_index);
    assert_eq!(z("calc(1 + 1)"), Some(Value::Specified(ZIndex::Value(2))));
    assert_eq!(z("calc(2.5)"), Some(Value::Specified(ZIndex::Value(3))));
    assert_eq!(z("calc(-2.5)"), Some(Value::Specified(ZIndex::Value(-2))));
    assert_eq!(
        z("max(-3, 7 / 2)"),
        Some(Value::Specified(ZIndex::Value(4)))
    );
    assert_eq!(
        z("calc(infinity)"),
        Some(Value::Specified(ZIndex::Value(i32::MAX)))
    );
    assert_eq!(z("calc(0 / 0)"), Some(Value::Specified(ZIndex::Value(0))));
    assert_eq!(z("calc(50%)"), None);
    assert_eq!(z("calc(1deg)"), None);
}

/// Properties and Values 1 §2.4 / Values 4 §10: a registered `<number>`,
/// `<integer>` or `<percentage>` takes a math function of that type.
#[test]
fn registered_numeric_syntaxes_take_math_functions() {
    let syntax = |s: &str| PropertySyntax::parse(s).unwrap();
    assert!(syntax("<number>").matches("calc(1 / 3)"));
    assert!(syntax("<number>").matches("sign(-2)"));
    assert!(!syntax("<number>").matches("calc(1% + 1)"));
    assert!(syntax("<integer>").matches("calc(1 + 1)"));
    assert!(syntax("<integer>").matches("round(2.4)"));
    assert!(!syntax("<integer>").matches("calc(1deg)"));
    assert!(syntax("<percentage>").matches("calc(10% + 5%)"));
    assert!(syntax("<percentage>").matches("min(10%, 20%)"));
    assert!(!syntax("<percentage>").matches("calc(10% + 5)"));
}

/// §10.9: NaN at the top level is 0 and an infinity clamps, so an
/// `<angle>` (a registered `@property` one, a hue) is always finite.
#[test]
fn angles_are_finite() {
    let angle = |v: &str| parse_angle(&tokenize(v).unwrap());
    assert_eq!(angle("calc(NaN * 1deg)"), Some(0.0));
    let inf = angle("calc(infinity * 1deg)").unwrap();
    assert!(inf.is_finite() && inf > 0.0, "{inf}");
    let neg = angle("calc(-infinity * 1deg)").unwrap();
    assert_eq!(neg, -inf);
    assert_eq!(angle("calc(1deg / 0)"), Some(inf));
}
