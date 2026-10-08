//! `calc-size(<calc-size-basis>, <calc-sum>)` (CSS Values 5 §10) and
//! `interpolate-size` (§11).
//!
//! The basis is a sizing keyword (`auto`, `min-content`, `max-content`,
//! `fit-content`, `fit-content(<l>)`), `any`, a `<length-percentage>` or a
//! nested `calc-size()`; the sum is a `<calc-sum>` that may name `size`,
//! the basis's resolved value. rdom takes sums linear in `size`
//! (`size * 0.5 + 2`, `size - 10%`), which every interpolation produces;
//! one with `size` inside a math function (`min(size, 10)`) or multiplied
//! by itself is invalid here (DIVERGENCES §3).

use super::numeric::{LengthPercentage, Range, length_percentage};
use crate::calc::{CalcExpr, CalcOp, ResolveCtx};
use crate::layout::{CalcSize, CalcSizeBasis, InterpolateSize, Size};
use crate::parse::token::Token;

/// `interpolate-size: numeric-only | allow-keywords`.
pub fn parse_interpolate_size(value: &[Token]) -> Option<InterpolateSize> {
    let [Token::Ident(kw)] = value else {
        return None;
    };
    match kw.to_ascii_lowercase().as_str() {
        "numeric-only" => Some(InterpolateSize::NumericOnly),
        "allow-keywords" => Some(InterpolateSize::AllowKeywords),
        _ => None,
    }
}

/// `calc-size(…)` as a `width` / `height` value: a [`Size::CalcSize`], or
/// the plain size it folds to (an `any` or length basis).
pub fn parse_calc_size(value: &[Token]) -> Option<Size> {
    let [Token::Function(name), inner @ .., Token::RParen] = value else {
        return None;
    };
    if !name.eq_ignore_ascii_case("calc-size") {
        return None;
    }
    let comma = top_level_comma(inner)?;
    let (basis, sum) = (&inner[..comma], &inner[comma + 1..]);
    let (factor, offset) = linear_sum(sum)?;
    Some(match basis_of(basis)? {
        Basis::Keyword(basis) => {
            Size::CalcSize(std::sync::Arc::new(CalcSize::new(basis, factor, offset)))
        }
        // `size` names nothing under `any`: the sum alone.
        Basis::Any if factor == 0.0 => plain(offset),
        Basis::Any => return None,
        // `size` is the length: `length * factor + offset`.
        Basis::Length(l) => plain(CalcExpr::binary(
            CalcOp::Add,
            CalcExpr::binary(CalcOp::Mul, l, CalcExpr::Number(factor)),
            offset,
        )),
        // A nested `calc-size()`: its `size` is ours.
        Basis::Nested(inner) => Size::CalcSize(std::sync::Arc::new(CalcSize::new(
            inner.basis.clone(),
            inner.factor * factor,
            CalcExpr::binary(
                CalcOp::Add,
                CalcExpr::binary(CalcOp::Mul, inner.offset.clone(), CalcExpr::Number(factor)),
                offset,
            ),
        ))),
    })
}

/// The size a math function is: cells when it holds no percentage.
fn plain(e: CalcExpr) -> Size {
    match e.linear_parts() {
        Some((cells, 0.0)) => {
            Size::Fixed(crate::calc::to_cells(cells).clamp(0, i32::from(u16::MAX)) as u16)
        }
        _ => Size::calc(e),
    }
}

enum Basis {
    Keyword(CalcSizeBasis),
    Any,
    Length(CalcExpr),
    Nested(CalcSize),
}

fn basis_of(tokens: &[Token]) -> Option<Basis> {
    if let [Token::Ident(kw)] = tokens {
        if kw.eq_ignore_ascii_case("auto") {
            return Some(Basis::Keyword(CalcSizeBasis::Auto));
        }
        if kw.eq_ignore_ascii_case("any") {
            return Some(Basis::Any);
        }
    }
    match super::length::parse_size(tokens)? {
        Size::Intrinsic(k) => Some(Basis::Keyword(CalcSizeBasis::Intrinsic(k))),
        Size::CalcSize(c) => Some(Basis::Nested((*c).clone())),
        Size::Fixed(n) => Some(Basis::Length(CalcExpr::Length(i32::from(n)))),
        Size::Percent(p) => Some(Basis::Length(CalcExpr::Percent(f64::from(p)))),
        Size::Calc(e) => Some(Basis::Length((*e).clone())),
        Size::Auto | Size::Flex(_) => None,
    }
}

/// The index of the first comma outside a nested function.
fn top_level_comma(tokens: &[Token]) -> Option<usize> {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.checked_sub(1)?,
            Token::Comma if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// The sum as `size * factor + offset`: parsed with `size` at 0, 1 and 2
/// — equal steps (at any percentage basis) make it linear in `size`.
fn linear_sum(sum: &[Token]) -> Option<(f64, CalcExpr)> {
    let at = |size: i64| -> Option<CalcExpr> {
        let mut tokens = vec![Token::Function("calc".into())];
        tokens.extend(sum.iter().map(|t| match t {
            Token::Ident(s) if s.eq_ignore_ascii_case("size") => Token::Number(size),
            other => other.clone(),
        }));
        tokens.push(Token::RParen);
        Some(match length_percentage(&tokens, Range::Any)? {
            LengthPercentage::Integer(n) => CalcExpr::Length(n),
            // The fraction stays: the sum rounds once, as a whole.
            LengthPercentage::Cells(v) => CalcExpr::Number(v),
            LengthPercentage::Expr(e) => e,
        })
    };
    let (e0, e1, e2) = (at(0)?, at(1)?, at(2)?);
    let value = |e: &CalcExpr, basis| e.resolve_f64(&ResolveCtx::new(basis));
    let factor = value(&e1, 0) - value(&e0, 0);
    for basis in [0, 100] {
        let (v0, v1, v2) = (value(&e0, basis), value(&e1, basis), value(&e2, basis));
        let linear = ((v1 - v0) - factor).abs() < 1e-9 && ((v2 - v1) - factor).abs() < 1e-9;
        if !linear || !factor.is_finite() {
            return None;
        }
    }
    Some((factor, e0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::token::tokenize;

    fn size(src: &str) -> Option<Size> {
        parse_calc_size(&tokenize(src).unwrap())
    }

    /// CSS Values 5 §10: the basis and a sum linear in `size`.
    #[test]
    fn calc_size_parses_a_linear_sum() {
        let Some(Size::CalcSize(c)) = size("calc-size(auto, size * 0.5 + 2)") else {
            panic!("{:?}", size("calc-size(auto, size * 0.5 + 2)"));
        };
        assert_eq!(c.basis, CalcSizeBasis::Auto);
        assert_eq!(c.factor, 0.5);
        assert_eq!(c.resolve(10, 0), 7);
        let Some(Size::CalcSize(c)) = size("calc-size(max-content, size - 10%)") else {
            panic!()
        };
        assert_eq!(c.resolve(20, 40), 16, "20 - 10% of 40");
        assert_eq!(size("calc-size(any, 7)"), Some(Size::Fixed(7)));
        assert_eq!(size("calc-size(10, size * 2)"), Some(Size::Fixed(20)));
        let Some(Size::CalcSize(c)) = size("calc-size(calc-size(auto, size + 1), size * 2)") else {
            panic!()
        };
        assert_eq!(c.resolve(3, 0), 8, "(3 + 1) * 2");
        assert_eq!(size("calc-size(auto, size * size)"), None, "not linear");
        assert_eq!(size("calc-size(any, size)"), None, "`size` under `any`");
        assert_eq!(size("calc-size(auto)"), None);
    }
}
