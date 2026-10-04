//! Tests for the `calc()` AST and its resolution.
use super::*;

fn cx(basis: i32) -> ResolveCtx {
    ResolveCtx::new(basis)
}

#[test]
fn number_resolves_to_self() {
    assert_eq!(CalcExpr::Number(5.0).resolve(&cx(100)), 5);
}

#[test]
fn length_resolves_to_cell_count() {
    assert_eq!(CalcExpr::Length(7).resolve(&cx(100)), 7);
}

#[test]
fn percent_resolves_against_basis() {
    assert_eq!(CalcExpr::Percent(50.0).resolve(&cx(100)), 50);
    assert_eq!(CalcExpr::Percent(50.0).resolve(&cx(40)), 20);
    assert_eq!(CalcExpr::Percent(25.0).resolve(&cx(80)), 20);
}

#[test]
fn add_percent_and_length_resolves_against_basis() {
    // calc(50% + 2) where basis = 40 → 22.
    let e = CalcExpr::binary(CalcOp::Add, CalcExpr::Percent(50.0), CalcExpr::Length(2));
    assert_eq!(e.resolve(&cx(40)), 22);
}

#[test]
fn sub_basis_minus_length() {
    // calc(100% - 4) where basis = 40 → 36.
    let e = CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(100.0), CalcExpr::Length(4));
    assert_eq!(e.resolve(&cx(40)), 36);
}

#[test]
fn mul_basis_by_number() {
    // calc(50% * 2) where basis = 40 → 40.
    let e = CalcExpr::binary(CalcOp::Mul, CalcExpr::Percent(50.0), CalcExpr::Number(2.0));
    assert_eq!(e.resolve(&cx(40)), 40);
}

#[test]
fn div_basis_by_number() {
    // calc(100% / 2) where basis = 40 → 20.
    let e = CalcExpr::binary(CalcOp::Div, CalcExpr::Percent(100.0), CalcExpr::Number(2.0));
    assert_eq!(e.resolve(&cx(40)), 20);
}

/// IEEE-754 division (CSS Values 4 §10.9): `10 / 0` is +∞, which the
/// top level clamps to the range.
#[test]
fn div_by_zero_is_infinite_and_clamps() {
    let e = CalcExpr::binary(CalcOp::Div, CalcExpr::Length(10), CalcExpr::Number(0.0));
    assert_eq!(e.resolve_f64(&cx(100)), f64::INFINITY);
    assert_eq!(e.resolve(&cx(100)), i32::MAX);
}

#[test]
fn contains_percent_walks_subtree() {
    let constant = CalcExpr::binary(CalcOp::Add, CalcExpr::Length(3), CalcExpr::Length(4));
    assert!(!constant.contains_percent());

    let withp = CalcExpr::binary(
        CalcOp::Add,
        CalcExpr::Length(3),
        CalcExpr::binary(CalcOp::Mul, CalcExpr::Percent(50.0), CalcExpr::Number(1.0)),
    );
    assert!(withp.contains_percent());
}

#[test]
fn half_to_even_rounding() {
    // 0.5 → 0, 1.5 → 2, 2.5 → 2, 3.5 → 4 (banker's rounding)
    assert_eq!(round_half_to_even(0.5), 0);
    assert_eq!(round_half_to_even(1.5), 2);
    assert_eq!(round_half_to_even(2.5), 2);
    assert_eq!(round_half_to_even(3.5), 4);
}

// ── C2-STEPPED ───────────────────────────────────────────────────────

fn eval(func: MathFunction, args: &[f64]) -> f64 {
    let args = args.iter().map(|v| CalcExpr::Number(*v)).collect();
    CalcExpr::function(func, args).resolve_f64(&cx(0))
}

/// CSS Values 4 §10.3.1: the rounding strategies, ties of `nearest`
/// toward +∞, B's sign ignored, and the argument-range rules — a zero
/// step is NaN; an infinite A stays infinite; an infinite B rounds a
/// finite A to zero (`nearest` / `to-zero`) or to the infinity in the
/// strategy's direction.
#[test]
fn round_strategies_and_edges() {
    use RoundingStrategy::*;
    let round = |s, a: f64, b: f64| eval(MathFunction::Round(s), &[a, b]);
    assert_eq!(round(Nearest, 2.5, 1.0), 3.0);
    assert_eq!(round(Nearest, -2.5, 1.0), -2.0);
    assert_eq!(round(Nearest, 7.0, -5.0), 5.0);
    assert_eq!(round(Up, -7.0, 5.0), -5.0);
    assert_eq!(round(Down, -7.0, 5.0), -10.0);
    assert_eq!(round(ToZero, -7.0, 5.0), -5.0);
    assert_eq!(round(Down, 10.0, 5.0), 10.0, "an exact multiple is itself");
    assert!(round(Nearest, 1.0, 0.0).is_nan());
    assert_eq!(round(Up, f64::INFINITY, 2.0), f64::INFINITY);
    assert!(round(Up, f64::INFINITY, f64::INFINITY).is_nan());
    assert_eq!(round(Nearest, 5.0, f64::INFINITY), 0.0);
    assert_eq!(round(Up, 5.0, f64::INFINITY), f64::INFINITY);
    assert_eq!(round(Down, 5.0, f64::INFINITY), 0.0);
    assert_eq!(round(Down, -5.0, f64::INFINITY), f64::NEG_INFINITY);
    assert_eq!(
        eval(MathFunction::Round(Nearest), &[2.5]),
        3.0,
        "B defaults to 1"
    );
}

/// CSS Values 4 §10.3.2 / §10.7: `mod()` has B's sign, `rem()` A's; a
/// zero B or an infinite A is NaN; an infinite B leaves A unless (for
/// `mod()`) the signs differ. `abs()` / `sign()` keep zero's sign.
#[test]
fn mod_rem_abs_sign() {
    assert_eq!(eval(MathFunction::Mod, &[-7.0, 5.0]), 3.0);
    assert_eq!(eval(MathFunction::Mod, &[7.0, -5.0]), -3.0);
    assert_eq!(eval(MathFunction::Rem, &[-7.0, 5.0]), -2.0);
    assert_eq!(eval(MathFunction::Rem, &[7.0, -5.0]), 2.0);
    assert!(eval(MathFunction::Mod, &[1.0, 0.0]).is_nan());
    assert!(eval(MathFunction::Rem, &[f64::INFINITY, 2.0]).is_nan());
    assert_eq!(eval(MathFunction::Rem, &[-3.0, f64::INFINITY]), -3.0);
    assert_eq!(eval(MathFunction::Mod, &[3.0, f64::INFINITY]), 3.0);
    assert!(eval(MathFunction::Mod, &[-3.0, f64::INFINITY]).is_nan());
    assert_eq!(eval(MathFunction::Abs, &[-4.5]), 4.5);
    assert_eq!(eval(MathFunction::Sign, &[-4.5]), -1.0);
    assert_eq!(eval(MathFunction::Sign, &[0.0]), 0.0);
    assert!(eval(MathFunction::Sign, &[-0.0]).is_sign_negative());
    assert_eq!(eval(MathFunction::Sign, &[9.0]), 1.0);
}

/// C2G-VIEWPORT-FIELDS: the cascade makes viewport units absolute (CSS
/// Values 4 §6.1.2), so layout's context has no viewport; a viewport
/// unit evaluated there is a computed-style field the cascade missed.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reached layout")]
fn a_viewport_unit_in_layout_asserts() {
    let e = CalcExpr::Dimension {
        value: 10.0,
        unit: CalcUnit::parse("vw").unwrap(),
    };
    e.resolve(&cx(0));
}

/// With a viewport the unit resolves: 10vw of 80 columns is 8.
#[test]
fn a_viewport_unit_with_a_viewport_resolves() {
    let e = CalcExpr::Dimension {
        value: 10.0,
        unit: CalcUnit::parse("vw").unwrap(),
    };
    assert_eq!(e.resolve(&cx(0).with_viewport(Viewport::new(80, 20))), 8);
}
