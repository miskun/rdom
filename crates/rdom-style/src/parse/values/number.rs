//! Scalar numeric values: `opacity`, unsigned cell counts (with a
//! constant `calc()`), `z-index` and `aspect-ratio`.

use super::calc::{looks_like_calc, parse_calc};
use super::numeric::{Range, components, number};
use crate::layout::ZIndex;
use crate::parse::token::Token;

/// `opacity: <number> | <percentage>` (a math function of type
/// `<number>` included) — clamped to `0..=1` per CSS
/// Color 4 §11.1; a percentage is the number divided by 100. The
/// tokenizer delivers the literal whole (`Number` for integers, `Float`
/// otherwise), so `0.05` is 0.05.
pub fn parse_opacity(value: &[Token]) -> Option<f32> {
    // Out-of-range values (including negatives, which arrive as
    // `Delim('-')` + literal) are valid and clamp — CSS Color 4 §11.1.
    let n = match value {
        [Token::Percentage(p)] => *p / 100.0,
        [Token::Delim('-'), Token::Percentage(p)] => -*p / 100.0,
        _ => number(value, Range::Any)?,
    };
    Some((n as f32).clamp(0.0, 1.0))
}

pub fn parse_unsigned(value: &[Token]) -> Option<u16> {
    if value.len() == 1
        && let Token::Number(n) = &value[0]
    {
        // Out of `u16` range (or negative) is invalid, not wrapped.
        return u16::try_from(*n).ok();
    }
    // Constant `calc(...)` — a percent-bearing form has no
    // sensible static basis here (padding/margin/gap don't carry
    // a Calc-bearing type), so we reject it. The block parser's
    // warning channel surfaces the rejection.
    if looks_like_calc(value) {
        let expr = parse_calc(value)?;
        if expr.contains_percent() {
            return None;
        }
        let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
        return Some(cells.max(0).min(u16::MAX as i32) as u16);
    }
    None
}

/// `auto` keyword | signed integer.
pub fn parse_z_index(value: &[Token]) -> Option<ZIndex> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(ZIndex::Auto),
        [Token::Number(n)] => i16::try_from(*n).ok().map(ZIndex::Value),
        [Token::Delim('-'), Token::Number(n)] => i16::try_from(-*n).ok().map(ZIndex::Value),
        _ => None,
    }
}

/// `aspect-ratio: auto || <ratio>` (CSS Sizing 4 §5.1), `<ratio> =
/// <number [0,∞]> [ / <number [0,∞]> ]?` (CSS Values 4 §5.7). `auto`
/// alone is `None` (no ratio).
pub fn parse_aspect_ratio(value: &[Token]) -> Option<Option<crate::layout::AspectRatio>> {
    let is_auto = |c: &[Token]| matches!(c, [Token::Ident(s)] if s.eq_ignore_ascii_case("auto"));
    let parts = components(value)?;
    let (auto, ratio) = match parts.as_slice() {
        [first, rest @ ..] if is_auto(first) => (true, rest),
        [rest @ .., last] if is_auto(last) => (true, rest),
        all => (false, all),
    };
    let term = |c: &[Token]| number(c, Range::NonNegative).map(|v| v as f32);
    let (numerator, denominator) = match ratio {
        [] if auto => return Some(None),
        [n] => (term(n)?, 1.0),
        [n, [Token::Delim('/')], d] => (term(n)?, term(d)?),
        _ => return None,
    };
    let ratio = crate::layout::AspectRatio::new(numerator, denominator)?;
    Some(Some(ratio.with_auto(auto)))
}

#[cfg(test)]
mod number_value_tests {
    use super::*;
    use crate::parse::token::tokenize;

    fn t(src: &str) -> Vec<Token> {
        tokenize(src).unwrap()
    }

    #[test]
    fn opacity_keeps_leading_fraction_zeros() {
        assert_eq!(parse_opacity(&t("0.05")), Some(0.05));
        assert_eq!(parse_opacity(&t(".5")), Some(0.5));
        assert_eq!(parse_opacity(&t("1")), Some(1.0));
        assert_eq!(parse_opacity(&t("2.5")), Some(1.0), "clamped");
        assert_eq!(
            parse_opacity(&t("-0.5")),
            Some(0.0),
            "out of range clamps (CSS Color 4)"
        );
    }
}
