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

// ── C2G-CALC-DEPTH ───────────────────────────────────────────────────

fn tokens(src: &str) -> Vec<Token> {
    crate::parse::token::tokenize(src).unwrap()
}

/// `calc(` opened `n` times around `1`.
fn nested_calcs(n: usize) -> String {
    format!("{}1{}", "calc(".repeat(n), ")".repeat(n))
}

/// CSS Values 4 §10 sets no nesting limit, and math functions take
/// attribute data (`attr()` with `type(<length>)`, Values 5 §8.7), so
/// the parser bounds its recursion: [`MAX_CALC_NESTING`] levels of math
/// functions and parentheses parse, one more is invalid.
#[test]
fn nesting_up_to_the_cap_parses_and_one_more_is_invalid() {
    let ok = parse_calc(&tokens(&nested_calcs(MAX_CALC_NESTING))).expect("at the cap");
    assert_eq!(ok.resolve(&crate::calc::ResolveCtx::new(0)), 1);
    assert_eq!(
        parse_calc(&tokens(&nested_calcs(MAX_CALC_NESTING + 1))),
        None
    );
    let parens = format!(
        "calc({}1{})",
        "(".repeat(MAX_CALC_NESTING),
        ")".repeat(MAX_CALC_NESTING)
    );
    assert_eq!(parse_calc(&tokens(&parens)), None, "parentheses count too");
}

/// Hostile nesting far past the cap is rejected without exhausting the
/// stack (it used to recurse once per level and abort the process).
#[test]
fn deep_nesting_is_rejected_without_overflow() {
    assert_eq!(parse_calc(&tokens(&nested_calcs(100_000))), None);
    let parens = format!("calc({}1{})", "(".repeat(100_000), ")".repeat(100_000));
    assert_eq!(parse_calc(&tokens(&parens)), None);
}

/// A flat chain builds a left-deep tree as deep as it is long, and every
/// walker — the type check, evaluation, viewport folding, drop —
/// recurses down it. A 100 000-term `1 + 1 + …` is rejected (deeper than
/// [`MAX_CALC_DEPTH`]) without overflowing; a chain within the cap
/// parses and evaluates.
#[test]
fn long_flat_chains_do_not_overflow() {
    let chain = |n: usize| format!("calc({})", vec!["1"; n].join(" + "));
    assert_eq!(parse_calc(&tokens(&chain(100_000))), None);
    let products = format!("calc({})", vec!["1"; 100_000].join(" * "));
    assert_eq!(parse_calc(&tokens(&products)), None);
    let within = parse_calc(&tokens(&chain(MAX_CALC_DEPTH))).expect("within the cap");
    assert_eq!(
        within.resolve(&crate::calc::ResolveCtx::new(0)),
        MAX_CALC_DEPTH as i32
    );
}
