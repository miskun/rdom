//! Sizes and lengths: `width` / `height` sizes (cells, `fr`, percent,
//! `calc()`), the `flex` shorthand, `min-*` sizes, signed `Length`s
//! for offsets and the `inset` shorthand.

use super::calc::{looks_like_calc, parse_calc};
use crate::layout::{Length, Size};
use crate::parse::token::Token;

/// A percentage literal, fraction intact, rejecting negative and
/// absurd values (`Size::percent_of` multiplies by an `i32` basis).
fn percent_fraction(p: f64) -> Option<f32> {
    (p >= 0.0 && p <= f64::from(u16::MAX)).then_some(p as f32)
}

pub fn parse_size(value: &[Token]) -> Option<Size> {
    // `auto` | `<n>` | `<n>fr` | `<n>%` | `calc(<expr>)`
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Size::Auto),
        [Token::Number(n)] if *n >= 0 => u16::try_from(*n).ok().map(Size::Fixed),
        [Token::Number(n), Token::Ident(unit)] if *n >= 0 && unit.eq_ignore_ascii_case("fr") => {
            u16::try_from(*n).ok().map(Size::Flex)
        }
        [Token::Percentage(n)] if *n >= 0.0 => Some(Size::Percent(percent_fraction(*n)?)),
        // calc(...) — parse to a CalcExpr. If the expression has
        // no percentages, constant-fold at parse time to Fixed.
        // Otherwise carry the AST through to layout via Size::Calc.
        _ if looks_like_calc(value) => parse_calc_to_size(value),
        _ => None,
    }
}

/// Parse a `calc(...)` value in `Size` position. Constant-fold to
/// `Size::Fixed` when the expression contains no percentages
/// (saves layout-time work for the common arithmetic-only case).
/// Otherwise carry the AST through as `Size::Calc` for layout
/// resolution.
fn parse_calc_to_size(value: &[Token]) -> Option<Size> {
    let expr = parse_calc(value)?;
    if expr.contains_percent() {
        Some(Size::Calc(Box::new(expr)))
    } else {
        let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
        let clamped = cells.max(0).min(u16::MAX as i32) as u16;
        Some(Size::Fixed(clamped))
    }
}

/// Parse the CSS `flex` shorthand. Models the main-axis sizing
/// of a flex child. Returns the `Size` that should be applied to
/// the child's width AND height (cross-axis `Size::Flex` already
/// means "stretch to container" in our layout, matching CSS
/// default `align-items: stretch` behavior).
///
/// Supported value shapes:
/// - `flex: auto`   → `Size::Flex(1)` (grow as `flex: 1 1 auto`)
/// - `flex: none`   → `Size::Auto`     (don't grow as `flex: 0 0 auto`)
/// - `flex: <n>`    → `n > 0` → `Size::Flex(n)`; `n == 0` → `Size::Auto`
/// - `flex: <n> <m> <basis>` → use `<n>` as the grow value; `<m>`
///   (shrink) and `<basis>` are parsed-and-accepted but ignored
///   until full flex-grow / flex-shrink / flex-basis tracking lands
///   in the substrate.
pub fn parse_flex_shorthand(value: &[Token]) -> Option<Size> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Size::Flex(1)),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(Size::Auto),
        [Token::Number(n)] if *n >= 0 => {
            if *n == 0 {
                Some(Size::Auto)
            } else {
                u16::try_from(*n).ok().map(Size::Flex)
            }
        }
        // Two-value form: `<grow> <shrink>` (basis defaults to 0).
        // Three-value form: `<grow> <shrink> <basis>`. Both ignore
        // shrink and basis for now; the grow value drives the Size.
        [Token::Number(n), _rest @ ..] if *n >= 0 && !value.is_empty() => {
            // Validate the remaining tokens look like a valid
            // flex shorthand tail (1 or 2 more numeric/auto values).
            // If not, reject so authors get a warning rather than a
            // silent partial-apply.
            // `<shrink>` is a non-negative number (integer or fractional);
            // `<basis>` is `auto`, a cell count, or a percentage — the
            // canonical `flex: 1 1 0%` included.
            let is_factor = |t: &Token| match t {
                Token::Number(n) => *n >= 0,
                Token::Float(f) => *f >= 0.0,
                _ => false,
            };
            let is_basis = |t: &Token| match t {
                Token::Number(n) => *n >= 0,
                Token::Percentage(p) => *p >= 0.0,
                Token::Ident(s) => s.eq_ignore_ascii_case("auto"),
                _ => false,
            };
            let tail = &value[1..];
            let tail_ok = match tail.len() {
                1 => is_factor(&tail[0]) || is_basis(&tail[0]),
                2 => is_factor(&tail[0]) && is_basis(&tail[1]),
                _ => false,
            };
            if !tail_ok {
                return None;
            }
            if *n == 0 {
                Some(Size::Auto)
            } else {
                u16::try_from(*n).ok().map(Size::Flex)
            }
        }
        _ => None,
    }
}

/// `min-width` / `min-height` value: `auto` | `<unsigned-int>`. The
/// `auto` keyword opts a flex item into intrinsic min-content
/// protection (decision 4 from the M5 pre-prep, M5.1.b).
pub fn parse_min_size(value: &[Token]) -> Option<crate::layout::MinSize> {
    use crate::layout::MinSize;
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(MinSize::Auto),
        [Token::Number(n)] if *n >= 0 => u16::try_from(*n).ok().map(MinSize::Cells),
        _ => None,
    }
}

/// `auto` keyword | signed integer in cells | `calc(<expr>)`.
pub fn parse_length(value: &[Token]) -> Option<Length> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Length::Auto),
        [Token::Number(n)] => Some(Length::Cells(*n)),
        [Token::Delim('-'), Token::Number(n)] => Some(Length::Cells(-*n)),
        _ if looks_like_calc(value) => parse_calc_to_length(value),
        _ => None,
    }
}

/// Parse a `calc(...)` value in `Length` position. Constant-fold
/// to `Length::Cells` when the expression has no percentages;
/// carry the AST through as `Length::Calc` otherwise.
fn parse_calc_to_length(value: &[Token]) -> Option<Length> {
    let expr = parse_calc(value)?;
    if expr.contains_percent() {
        Some(Length::Calc(Box::new(expr)))
    } else {
        Some(Length::Cells(
            expr.resolve(&crate::calc::ResolveCtx::new(0)),
        ))
    }
}

/// `inset: <a> [<b> [<c> [<d>]]]` — same clockwise expansion as
/// `padding`, but each value can be `auto` or signed (negative).
pub fn parse_inset_shorthand(value: &[Token]) -> Option<(Length, Length, Length, Length)> {
    let lengths = split_lengths(value)?;
    let p = match lengths.as_slice() {
        [a] => (a.clone(), a.clone(), a.clone(), a.clone()),
        [a, b] => (a.clone(), b.clone(), a.clone(), b.clone()),
        [a, b, c] => (a.clone(), b.clone(), c.clone(), b.clone()),
        [a, b, c, d] => (a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return None,
    };
    Some(p)
}

/// Split a value-token slice into 1..=4 `Length` values separated
/// by whitespace (already eaten by the tokenizer). Used by the
/// `inset` shorthand.
fn split_lengths(value: &[Token]) -> Option<Vec<Length>> {
    let mut out = Vec::with_capacity(4);
    let mut i = 0usize;
    while i < value.len() {
        // Try the two-token negative pattern first.
        if let (Some(Token::Delim('-')), Some(Token::Number(n))) = (value.get(i), value.get(i + 1))
        {
            out.push(Length::Cells(-*n));
            i += 2;
            continue;
        }
        let l = match value.get(i)? {
            Token::Ident(s) if s.eq_ignore_ascii_case("auto") => Length::Auto,
            Token::Number(n) => Length::Cells(*n),
            _ => return None,
        };
        out.push(l);
        i += 1;
        if out.len() > 4 {
            return None;
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(test)]
mod number_value_tests {
    use super::*;
    use crate::calc::{CalcExpr, CalcOp};
    use crate::parse::token::tokenize;

    fn t(src: &str) -> Vec<Token> {
        tokenize(src).unwrap()
    }

    #[test]
    fn fractional_percent_in_calc_and_size() {
        let e = parse_calc(&t("calc(12.5% + 1)")).unwrap();
        assert_eq!(
            e,
            CalcExpr::Binary {
                op: CalcOp::Add,
                lhs: Box::new(CalcExpr::Percent(12.5)),
                rhs: Box::new(CalcExpr::Number(1.0)),
            }
        );
        // Integer cells: a fractional percentage width truncates to whole percent.
        assert_eq!(parse_size(&t("50%")), Some(Size::Percent(50.0)));
        assert_eq!(parse_size(&t("12.5%")), Some(Size::Percent(12.5)));
    }

    /// Out-of-range integers are rejected (declaration dropped), not
    /// silently zero.
    #[test]
    fn oversized_size_is_rejected_not_zero() {
        assert_eq!(parse_size(&t("99999999999")), None);
        assert_eq!(parse_size(&t("70000")), None, "u16 range");
    }
}
