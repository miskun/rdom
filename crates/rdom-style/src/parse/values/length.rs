//! Sizes and lengths: `width` / `height` sizes (cells, `fr`, percent,
//! `calc()`), the `flex` shorthand, `min-*` sizes, signed `Length`s
//! for offsets and the `inset` shorthand.

use super::numeric::{
    LengthPercentage, Range, cells_i32, cells_u16, components, length_percentage, number,
};
use crate::calc::CalcExpr;
use crate::layout::{Length, MaxSize, MinSize, Size};
use crate::parse::token::Token;

/// A percentage literal, fraction intact, rejecting negative and
/// absurd values (`Size::percent_of` multiplies by an `i32` basis).
fn percent_fraction(p: f64) -> Option<f32> {
    (p >= 0.0 && p <= f64::from(u16::MAX)).then_some(p as f32)
}

pub fn parse_size(value: &[Token]) -> Option<Size> {
    // `auto` | `<n>fr` | `<length-percentage [0,∞]>`
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Size::Auto),
        [Token::Dimension { value, unit, .. }] if unit.eq_ignore_ascii_case("fr") => {
            flex_factor(*value).map(Size::Flex)
        }
        _ => match length_percentage(value, Range::NonNegative)? {
            LengthPercentage::Integer(n) => u16::try_from(n).ok().map(Size::Fixed),
            LengthPercentage::Cells(v) => Some(Size::Fixed(cells_u16(v))),
            LengthPercentage::Expr(CalcExpr::Percent(p)) => {
                Some(Size::Percent(percent_fraction(p)?))
            }
            LengthPercentage::Expr(e) => Some(Size::Calc(Box::new(e))),
        },
    }
}

/// A flex factor (`<flex>` / `<number [0,∞]>`, CSS Flexbox §7.1 /
/// Grid §7.2.3): non-negative and finite, kept as written.
fn flex_factor(v: f64) -> Option<f32> {
    (v >= 0.0 && v <= f64::from(f32::MAX)).then_some(v as f32)
}

/// Parse the CSS `flex` shorthand. Models the main-axis sizing
/// of a flex child. Returns the `Size` that should be applied to
/// the child's width AND height (cross-axis `Size::Flex` already
/// means "stretch to container" in our layout, matching CSS
/// default `align-items: stretch` behavior).
///
/// Supported value shapes (factors are `<number [0,∞]>`, fractions
/// included — CSS Flexbox §7.1):
/// - `flex: auto`   → `Size::Flex(1.0)` (grow as `flex: 1 1 auto`)
/// - `flex: none`   → `Size::Auto`     (don't grow as `flex: 0 0 auto`)
/// - `flex: <n>`    → `n > 0` → `Size::Flex(n)`; `n == 0` → `Size::Auto`
/// - `flex: <n> <m> <basis>` → use `<n>` as the grow value; `<m>`
///   (shrink) and `<basis>` are parsed-and-accepted but ignored
///   until full flex-grow / flex-shrink / flex-basis tracking lands
///   in the substrate.
pub fn parse_flex_shorthand(value: &[Token]) -> Option<Size> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => return Some(Size::Flex(1.0)),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => return Some(Size::Auto),
        _ => {}
    }
    let parts = components(value)?;
    let (grow, tail) = parts.split_first()?;
    let grow = flex_factor(number(grow, Range::NonNegative)?)?;
    // `<shrink>` is a non-negative number; `<basis>` is `auto` or a
    // non-negative `<length-percentage>` — the canonical `flex: 1 1 0%`
    // included. A malformed tail rejects the whole declaration rather
    // than applying part of it.
    let is_factor = |c: &[Token]| number(c, Range::NonNegative).is_some();
    let is_basis = |c: &[Token]| {
        matches!(c, [Token::Ident(s)] if s.eq_ignore_ascii_case("auto"))
            || length_percentage(c, Range::NonNegative).is_some()
    };
    let tail_ok = match tail {
        [] => true,
        [a] => is_factor(a) || is_basis(a),
        [a, b] => is_factor(a) && is_basis(b),
        _ => false,
    };
    if !tail_ok {
        return None;
    }
    Some(if grow == 0.0 {
        Size::Auto
    } else {
        Size::Flex(grow)
    })
}

/// `flex-shrink: <number [0,∞]>` (CSS Flexbox §7.3.2).
pub fn parse_flex_factor(value: &[Token]) -> Option<f32> {
    flex_factor(number(value, Range::NonNegative)?)
}

/// `min-width` / `min-height` value: `auto` | `<length-percentage
/// [0,∞]>`. The `auto` keyword opts a flex item into intrinsic
/// min-content protection (decision 4 from the M5 pre-prep, M5.1.b).
pub fn parse_min_size(value: &[Token]) -> Option<MinSize> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(MinSize::Auto),
        _ => match length_percentage(value, Range::NonNegative)? {
            LengthPercentage::Integer(n) => u16::try_from(n).ok().map(MinSize::Cells),
            LengthPercentage::Cells(v) => Some(MinSize::Cells(cells_u16(v))),
            LengthPercentage::Expr(e) => Some(MinSize::Calc(Box::new(e))),
        },
    }
}

/// `max-width` / `max-height` value: `none | <length-percentage [0,∞]>`
/// (CSS Sizing 3 §5.2). `Some(None)` is `none`; `None` is invalid.
pub fn parse_max_size(value: &[Token]) -> Option<Option<MaxSize>> {
    if matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case("none")) {
        return Some(None);
    }
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(|n| Some(MaxSize::Cells(n))),
        LengthPercentage::Cells(v) => Some(Some(MaxSize::Cells(cells_u16(v)))),
        LengthPercentage::Expr(e) => Some(Some(MaxSize::Calc(Box::new(e)))),
    }
}

/// An inset (`top` / `right` / `bottom` / `left`): `auto` |
/// `<length-percentage>`, either sign (CSS Position 3 §3.1).
pub fn parse_length(value: &[Token]) -> Option<Length> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Length::Auto),
        _ => Some(match length_percentage(value, Range::Any)? {
            LengthPercentage::Integer(n) => Length::Cells(n),
            LengthPercentage::Cells(v) => Length::Cells(cells_i32(v)),
            LengthPercentage::Expr(e) => Length::Calc(Box::new(e)),
        }),
    }
}

/// `inset: <a> [<b> [<c> [<d>]]]` — same clockwise expansion as
/// `padding`; each value is an inset (`auto` or a signed
/// `<length-percentage>`).
pub fn parse_inset_shorthand(value: &[Token]) -> Option<(Length, Length, Length, Length)> {
    let lengths = components(value)?
        .into_iter()
        .map(parse_length)
        .collect::<Option<Vec<_>>>()?;
    let p = match lengths.as_slice() {
        [a] => (a.clone(), a.clone(), a.clone(), a.clone()),
        [a, b] => (a.clone(), b.clone(), a.clone(), b.clone()),
        [a, b, c] => (a.clone(), b.clone(), c.clone(), b.clone()),
        [a, b, c, d] => (a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return None,
    };
    Some(p)
}

#[cfg(test)]
mod number_value_tests {
    use super::*;
    use crate::calc::{CalcExpr, CalcOp};
    use crate::parse::token::tokenize;
    use crate::parse::values::parse_calc;

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
