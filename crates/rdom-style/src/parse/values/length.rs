//! Sizes and lengths: `width` / `height` sizes (cells, `fr`, percent,
//! `calc()`), the `flex` shorthand and `flex-basis`, `min-*` sizes, signed `Length`s
//! for offsets and the `inset` shorthand.

use super::numeric::{
    LengthPercentage, Range, cells_i32, cells_u16, components, length_percentage, number,
};
use crate::calc::CalcExpr;
use crate::layout::{FlexBasis, Length, MaxSize, MinSize, Size};
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

/// The `flex` shorthand's three longhands (CSS Flexbox §7.2).
#[derive(Debug, Clone, PartialEq)]
pub struct FlexShorthand {
    /// `flex-grow`: `<number [0,∞]>`.
    pub grow: f32,
    /// `flex-shrink`: `<number [0,∞]>`.
    pub shrink: f32,
    /// `flex-basis`.
    pub basis: FlexBasis,
}

/// Parse the `flex` shorthand (CSS Flexbox §7.2): `none | [
/// <'flex-grow'> <'flex-shrink'>? || <'flex-basis'> ]`. `none` is
/// `0 0 auto`; an omitted grow or shrink is 1 and an omitted basis 0
/// (so `flex: <n>` is `<n> 1 0`, and `auto` — a lone basis — is
/// `1 1 auto`). The basis may come first; a bare number is a factor
/// unless two factors precede it (a unitless zero included, §7.2).
pub fn parse_flex_shorthand(value: &[Token]) -> Option<FlexShorthand> {
    if matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case("none")) {
        return Some(FlexShorthand {
            grow: 0.0,
            shrink: 0.0,
            basis: FlexBasis::Auto,
        });
    }
    let factor = |c: &[Token]| flex_factor(number(c, Range::NonNegative)?);
    let parts = components(value)?;
    let (mut grow, mut shrink, mut basis) = (None, None, None);
    let mut rest = parts.as_slice();
    while let [first, tail @ ..] = rest {
        rest = tail;
        if grow.is_none()
            && let Some(g) = factor(first)
        {
            grow = Some(g);
            // `<'flex-grow'> <'flex-shrink'>?`: a factor right after
            // the grow is the shrink.
            if let [next, tail @ ..] = rest
                && let Some(s) = factor(next)
            {
                shrink = Some(s);
                rest = tail;
            }
        } else if basis.is_none() {
            basis = Some(parse_flex_basis(first)?);
        } else {
            return None;
        }
    }
    Some(FlexShorthand {
        grow: grow.unwrap_or(1.0),
        shrink: shrink.unwrap_or(1.0),
        basis: basis.unwrap_or(FlexBasis::Cells(0)),
    })
}

/// `flex-basis: content | <'width'>` (CSS Flexbox §7.3.3): `auto`,
/// `content` or a `<length-percentage [0,∞]>`.
fn parse_flex_basis(value: &[Token]) -> Option<FlexBasis> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(FlexBasis::Auto),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("content") => Some(FlexBasis::Content),
        _ => match length_percentage(value, Range::NonNegative)? {
            LengthPercentage::Integer(n) => u16::try_from(n).ok().map(FlexBasis::Cells),
            LengthPercentage::Cells(v) => Some(FlexBasis::Cells(cells_u16(v))),
            LengthPercentage::Expr(e) => Some(FlexBasis::Calc(Box::new(e))),
        },
    }
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
            // A percentage past `percent_fraction`'s range stays a `calc()`
            // (resolved the same way), as it parsed before `Percent`.
            LengthPercentage::Expr(CalcExpr::Percent(p)) if percent_fraction(p).is_some() => {
                Some(MinSize::Percent(p as f32))
            }
            LengthPercentage::Expr(e) => Some(MinSize::Calc(Box::new(e))),
        },
    }
}

/// `max-width` / `max-height` value: `none | <length-percentage [0,∞]>`
/// (CSS Sizing 3 §5.2). `None` is invalid.
pub fn parse_max_size(value: &[Token]) -> Option<MaxSize> {
    if matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case("none")) {
        return Some(MaxSize::None);
    }
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(MaxSize::Cells),
        LengthPercentage::Cells(v) => Some(MaxSize::Cells(cells_u16(v))),
        LengthPercentage::Expr(CalcExpr::Percent(p)) if percent_fraction(p).is_some() => {
            Some(MaxSize::Percent(p as f32))
        }
        LengthPercentage::Expr(e) => Some(MaxSize::Calc(Box::new(e))),
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
