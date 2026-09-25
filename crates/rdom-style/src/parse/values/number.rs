//! Scalar numeric values: `opacity`, unsigned cell counts (with a
//! constant `calc()`), `z-index` and `aspect-ratio`.

use super::calc::{looks_like_calc, parse_calc};
use crate::layout::ZIndex;
use crate::parse::token::Token;

/// `opacity: <number>` — clamped to `0..=1` per CSS Color 4 §11.1. The
/// tokenizer delivers the literal whole (`Number` for integers, `Float`
/// otherwise), so `0.05` is 0.05.
pub fn parse_opacity(value: &[Token]) -> Option<f32> {
    // Out-of-range values (including negatives, which arrive as
    // `Delim('-')` + literal) are valid and clamp — CSS Color 4 §11.1.
    let n = match value {
        [Token::Number(n)] => f64::from(*n),
        [Token::Float(f)] => *f,
        [Token::Delim('-'), Token::Number(n)] => -f64::from(*n),
        [Token::Delim('-'), Token::Float(f)] => -*f,
        _ => return None,
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

/// `aspect-ratio: <w> / <h>`. v1 surface: `<positive-int>/<positive-int>`
/// (e.g. `16/9`, `4/3`, `1/1`). Stored as the integer pair so the
/// round-trip recovers the original form. `auto` keyword and the
/// single-number CSS form are deferred polish.
pub fn parse_aspect_ratio(value: &[Token]) -> Option<crate::layout::AspectRatio> {
    match value {
        [Token::Number(w), Token::Delim('/'), Token::Number(h)] if *w > 0 && *h > 0 => {
            crate::layout::AspectRatio::new(u16::try_from(*w).ok()?, u16::try_from(*h).ok()?)
        }
        _ => None,
    }
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
