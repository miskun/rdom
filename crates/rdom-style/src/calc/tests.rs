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

#[test]
fn div_by_zero_saturates_to_zero() {
    let e = CalcExpr::binary(CalcOp::Div, CalcExpr::Length(10), CalcExpr::Number(0.0));
    assert_eq!(e.resolve(&cx(100)), 0);
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
