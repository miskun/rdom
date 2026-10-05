//! Scalar numeric values: `opacity`, `z-index` and `aspect-ratio`.

use super::calc::looks_like_calc;
use super::numeric::{Range, components, integer, number, number_or_percentage};
use crate::layout::ZIndex;
use crate::parse::token::Token;

/// `opacity: <number> | <percentage>` (a math function included, with
/// percentages as numbers: `min(1, 50%)`) — clamped to `0..=1` per CSS
/// Color 4 §11.1; a percentage is the number divided by 100. The
/// tokenizer delivers the literal whole (`Number` for integers, `Float`
/// otherwise), so `0.05` is 0.05.
pub fn parse_opacity(value: &[Token]) -> Option<f32> {
    // Out-of-range values (including negatives, which arrive as
    // `Delim('-')` + literal) are valid and clamp — CSS Color 4 §11.1.
    let n = number_or_percentage(value)?;
    Some((n as f32).clamp(0.0, 1.0))
}

/// `auto` keyword | `<integer>` (a math function rounded to one
/// included, CSS Values 4 §10.9). A literal outside `i16` is invalid; a
/// math function's result clamps to it.
pub fn parse_z_index(value: &[Token]) -> Option<ZIndex> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(ZIndex::Auto),
        _ if looks_like_calc(value) => {
            let n = integer(value)?.clamp(i64::from(i16::MIN), i64::from(i16::MAX));
            Some(ZIndex::Value(n as i16))
        }
        _ => i16::try_from(integer(value)?).ok().map(ZIndex::Value),
    }
}

/// `order: <integer>` (CSS Flexbox §5.4). A math function rounds to
/// an integer (CSS Values 4 §10.9) and a value past `i32` clamps to it
/// (§5.1).
pub fn parse_order(value: &[Token]) -> Option<i32> {
    integer(value).map(super::numeric::clamp_i32)
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
