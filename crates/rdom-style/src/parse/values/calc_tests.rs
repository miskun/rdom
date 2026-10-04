//! Tests for the math-function parser: grammar, precedence, constant
//! folding into the property types.

use super::*;
use crate::layout::{Length, Size};
use crate::parse::token::Token;
use crate::parse::values::{parse_length, parse_size};

fn calc_tokens(inner: Vec<Token>) -> Vec<Token> {
    let mut v = vec![Token::Function("calc".to_string())];
    v.extend(inner);
    v.push(Token::RParen);
    v
}

#[test]
fn bare_number() {
    let tokens = calc_tokens(vec![Token::Number(5)]);
    let e = parse_calc(&tokens).unwrap();
    assert_eq!(e, CalcExpr::Number(5.0));
}

#[test]
fn bare_percent() {
    let tokens = calc_tokens(vec![Token::Percentage(50.0)]);
    let e = parse_calc(&tokens).unwrap();
    assert_eq!(e, CalcExpr::Percent(50.0));
}

#[test]
fn add_percent_and_number() {
    let tokens = calc_tokens(vec![
        Token::Percentage(50.0),
        Token::Delim('+'),
        Token::Number(2),
    ]);
    let e = parse_calc(&tokens).unwrap();
    assert_eq!(
        e,
        CalcExpr::binary(CalcOp::Add, CalcExpr::Percent(50.0), CalcExpr::Number(2.0))
    );
}

#[test]
fn sub_full_minus_constant() {
    let tokens = calc_tokens(vec![
        Token::Percentage(100.0),
        Token::Delim('-'),
        Token::Number(4),
    ]);
    let e = parse_calc(&tokens).unwrap();
    assert_eq!(
        e,
        CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(100.0), CalcExpr::Number(4.0))
    );
}

#[test]
fn mul_binds_tighter_than_add() {
    // calc(2 + 3 * 4) → Add(2, Mul(3, 4))
    let tokens = calc_tokens(vec![
        Token::Number(2),
        Token::Delim('+'),
        Token::Number(3),
        Token::Delim('*'),
        Token::Number(4),
    ]);
    let e = parse_calc(&tokens).unwrap();
    let expected = CalcExpr::binary(
        CalcOp::Add,
        CalcExpr::Number(2.0),
        CalcExpr::binary(CalcOp::Mul, CalcExpr::Number(3.0), CalcExpr::Number(4.0)),
    );
    assert_eq!(e, expected);
}

#[test]
fn parens_override_precedence() {
    // calc((2 + 3) * 4) → Mul(Add(2,3), 4)
    let tokens = calc_tokens(vec![
        Token::LParen,
        Token::Number(2),
        Token::Delim('+'),
        Token::Number(3),
        Token::RParen,
        Token::Delim('*'),
        Token::Number(4),
    ]);
    let e = parse_calc(&tokens).unwrap();
    let expected = CalcExpr::binary(
        CalcOp::Mul,
        CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
        CalcExpr::Number(4.0),
    );
    assert_eq!(e, expected);
}

#[test]
fn nested_calc() {
    // calc(calc(2 + 3) * 4) — semantically same as the parens form.
    let tokens = calc_tokens(vec![
        Token::Function("calc".to_string()),
        Token::Number(2),
        Token::Delim('+'),
        Token::Number(3),
        Token::RParen,
        Token::Delim('*'),
        Token::Number(4),
    ]);
    let e = parse_calc(&tokens).unwrap();
    let expected = CalcExpr::binary(
        CalcOp::Mul,
        CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
        CalcExpr::Number(4.0),
    );
    assert_eq!(e, expected);
}

#[test]
fn unary_minus() {
    let tokens = calc_tokens(vec![
        Token::Number(5),
        Token::Delim('-'),
        Token::Delim('-'),
        Token::Number(3),
    ]);
    // calc(5 - -3) = Sub(5, -3) — and -3 is a Number(-3.0).
    let e = parse_calc(&tokens).unwrap();
    assert_eq!(
        e,
        CalcExpr::binary(CalcOp::Sub, CalcExpr::Number(5.0), CalcExpr::Number(-3.0))
    );
}

#[test]
fn invalid_form_returns_none() {
    // Missing closing paren.
    let tokens = vec![Token::Function("calc".to_string()), Token::Number(5)];
    assert!(parse_calc(&tokens).is_none());

    // Trailing tokens after the calc.
    let tokens = calc_tokens(vec![Token::Number(5)]);
    let mut with_trail = tokens.clone();
    with_trail.push(Token::Number(99));
    assert!(parse_calc(&with_trail).is_none());

    // Not a calc() at all.
    let tokens = vec![Token::Number(5)];
    assert!(parse_calc(&tokens).is_none());
}

#[test]
fn looks_like_calc_detects_function_token() {
    let yes = vec![Token::Function("calc".to_string())];
    let no = vec![Token::Number(5)];
    assert!(looks_like_calc(&yes));
    assert!(!looks_like_calc(&no));
}

// ─── Parse-time constant-eval integration ───────────────────────

#[test]
fn parse_size_accepts_constant_calc() {
    // `width: calc(2 + 3)` → `Size::Fixed(5)`.
    let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('+'), Token::Number(3)]);
    assert_eq!(parse_size(&tokens), Some(Size::Fixed(5)));
}

#[test]
fn parse_size_accepts_constant_calc_with_precedence() {
    // `width: calc(2 + 3 * 4)` → `Size::Fixed(14)`.
    let tokens = calc_tokens(vec![
        Token::Number(2),
        Token::Delim('+'),
        Token::Number(3),
        Token::Delim('*'),
        Token::Number(4),
    ]);
    assert_eq!(parse_size(&tokens), Some(Size::Fixed(14)));
}

#[test]
fn parse_size_carries_percent_bearing_calc_as_calc_variant() {
    // M6 full: percent-bearing calc parses into Size::Calc and
    // resolves at layout time.
    let tokens = calc_tokens(vec![
        Token::Percentage(100.0),
        Token::Delim('-'),
        Token::Number(4),
    ]);
    match parse_size(&tokens) {
        Some(Size::Calc(expr)) => {
            assert!(expr.contains_percent());
        }
        other => panic!("expected Size::Calc, got {other:?}"),
    }
}

#[test]
fn parse_size_clamps_negative_constant_calc_to_zero() {
    // `width: calc(2 - 10)` → -8 cells → clamped to 0.
    let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('-'), Token::Number(10)]);
    assert_eq!(parse_size(&tokens), Some(Size::Fixed(0)));
}

#[test]
fn parse_length_accepts_constant_calc_negative_result() {
    // `top: calc(-3 * 2)` → -6 → Length::Cells(-6).
    let tokens = calc_tokens(vec![
        Token::Delim('-'),
        Token::Number(3),
        Token::Delim('*'),
        Token::Number(2),
    ]);
    assert_eq!(parse_length(&tokens), Some(Length::Cells(-6)));
}

#[test]
fn parse_length_carries_percent_bearing_calc_as_calc_variant() {
    let tokens = calc_tokens(vec![
        Token::Percentage(50.0),
        Token::Delim('+'),
        Token::Number(2),
    ]);
    match parse_length(&tokens) {
        Some(Length::Calc(expr)) => {
            assert!(expr.contains_percent());
        }
        other => panic!("expected Length::Calc, got {other:?}"),
    }
}
