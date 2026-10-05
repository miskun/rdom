//! The shared leaf of every length-bearing property:
//! `<length-percentage>` (CSS Values 4 §5.6), with rdom's unitless
//! integer cell as the length, and the splitter that cuts a
//! multi-value declaration (`padding: 1 10% calc(…)`) into its
//! component values.
//!
//! Each property parser (`width`, `padding-*`, `margin-*`, the insets,
//! `min-*` / `max-*`, `gap`) calls [`length_percentage`] and maps the
//! result onto its own storage type, so a unit or math function added
//! here reaches every property at once.

use super::calc::{looks_like_calc, parse_calc, parse_math};
use crate::calc::{CalcExpr, CalcKind, CalcUnit, ResolveCtx};
use crate::parse::token::Token;

/// Which signs a property accepts for a literal (CSS Values 4 §4.1:
/// a range restriction rejects an out-of-range literal; a math
/// function's result is clamped where it is used instead, §10.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Range {
    /// `[0,∞]` — sizes, padding, gaps.
    NonNegative,
    /// Any sign — margins, insets.
    Any,
}

/// One parsed `<length-percentage>`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LengthPercentage {
    /// A bare integer: rdom's cell (DIVERGENCES §1). Kept apart so a
    /// property can reject a literal outside its storage range rather
    /// than clamp it.
    Integer(i32),
    /// A length known at parse time, in (possibly fractional) cells — a
    /// unitless fraction (`1.5`), an absolute dimension, or a math
    /// function without a percentage. Rounded onto the grid where it
    /// becomes a property value.
    Cells(f64),
    /// A value that needs the layout's basis or the viewport: a
    /// percentage or a viewport-percentage length, alone or inside a
    /// math function.
    Expr(CalcExpr),
}

pub(crate) use crate::absolute::{cells_i32, cells_u16};

/// Parse one component value as a `<length-percentage>`: a bare
/// number (cells — an integer or a fraction, the same length
/// `calc(<number>)` is), a dimension in a length unit ([`CalcUnit`]), a
/// percentage, or a math function. A leading `-`
/// is the literal's sign (the tokenizer emits it as a delimiter).
pub(crate) fn length_percentage(component: &[Token], range: Range) -> Option<LengthPercentage> {
    let (negative, rest) = match component {
        [Token::Delim('-'), rest @ ..] => (true, rest),
        _ => (false, component),
    };
    if negative && range == Range::NonNegative {
        return None;
    }
    let sign = if negative { -1.0 } else { 1.0 };
    match rest {
        [Token::Number(n)] => Some(LengthPercentage::Integer(if negative { -*n } else { *n })),
        // rdom's unitless cell takes any `<number>`: `1.5` is the length
        // `calc(1.5)` is (C4G-NUMBER-RANGE).
        [Token::Float(f)] => Some(LengthPercentage::Cells(sign * *f)),
        [Token::Percentage(p)] => Some(LengthPercentage::Expr(CalcExpr::Percent(sign * *p))),
        [Token::Dimension { value, unit, .. }] => {
            let unit = CalcUnit::parse(unit).filter(|u| u.kind().is_length())?;
            let value = sign * *value;
            Some(if unit.needs_context() {
                LengthPercentage::Expr(CalcExpr::Dimension { value, unit })
            } else {
                LengthPercentage::Cells(
                    CalcExpr::Dimension { value, unit }.resolve_f64(&ResolveCtx::new(0)),
                )
            })
        }
        _ if !negative && looks_like_calc(rest) => {
            let expr = parse_calc(rest).filter(|e| e.kind().is_some_and(|k| k.is_length()))?;
            Some(if expr.contains_percent() || expr.needs_context() {
                LengthPercentage::Expr(expr)
            } else {
                LengthPercentage::Cells(expr.resolve_f64(&ResolveCtx::new(0)))
            })
        }
        _ => None,
    }
}

/// Parse one component value as a `<number>` (CSS Values 4 §5.4):
/// an integer or a fraction (`0.5`, `.5`, `1e3`), or a math function
/// of type `<number>` (`calc(1 / 3)`, `sin(1)`). A leading `-` is the
/// literal's sign.
pub(crate) fn number(component: &[Token], range: Range) -> Option<f64> {
    let (negative, rest) = match component {
        [Token::Delim('-'), rest @ ..] => (true, rest),
        _ => (false, component),
    };
    if negative && range == Range::NonNegative {
        return None;
    }
    let n = match rest {
        [Token::Number(n)] => f64::from(*n),
        [Token::Float(f)] => *f,
        _ if !negative && looks_like_calc(rest) => {
            let v = number_math(parse_calc(rest)?)?;
            // A range-restricted property clamps a computed value
            // rather than rejecting it.
            return Some(if range == Range::NonNegative {
                v.max(0.0)
            } else {
                v
            });
        }
        _ => return None,
    };
    Some(if negative { -n } else { n })
}

/// The value of a `<number>` math function, evaluated now (a number
/// property's value is known at parse time): `None` unless it is of type
/// `<number>` and holds nothing that needs a basis — a percentage (which
/// a `<number>` property does not take, CSS Values 4 §10.9) or a
/// viewport unit (`sign(10vw - 40)`; rdom resolves numbers at parse
/// time, DIVERGENCES). A NaN result is 0 (§10.9); an infinite one is
/// left for the property to clamp.
fn number_math(expr: CalcExpr) -> Option<f64> {
    if expr.kind() != Some(CalcKind::Number) || expr.contains_percent() || expr.needs_context() {
        return None;
    }
    let v = expr.resolve_f64(&ResolveCtx::new(0));
    Some(if v.is_nan() { 0.0 } else { v })
}

/// Parse a value as `<number> | <percentage>` where the percentage is a
/// number (`opacity`, CSS Color 4 §11.1: 50% is 0.5): a literal of
/// either, or a math function typed with percentages as numbers
/// ([`CalcExpr::kind_as_number`]) — `calc(50%)`, `min(1, 50%)`. A
/// leading `-` is the literal's sign.
pub(crate) fn number_or_percentage(value: &[Token]) -> Option<f64> {
    match value {
        [Token::Percentage(p)] => Some(*p / 100.0),
        [Token::Delim('-'), Token::Percentage(p)] => Some(-*p / 100.0),
        _ if looks_like_calc(value) => {
            let expr = parse_math(value)?;
            if expr.kind_as_number() != Some(CalcKind::Number) || expr.needs_context() {
                return None;
            }
            // A percentage is the number divided by 100: a basis of 1.
            let v = expr.resolve_f64(&ResolveCtx::new(1));
            Some(if v.is_nan() { 0.0 } else { v })
        }
        _ => number(value, Range::Any),
    }
}

/// Parse a value as an `<integer>` (CSS Values 4 §5.2): an integer
/// literal, or a math function of type `<number>` rounded to the nearest
/// integer, a half toward +∞ (§10.9). Infinities saturate; the caller
/// clamps to its property's range.
pub(crate) fn integer(value: &[Token]) -> Option<i64> {
    match value {
        [Token::Number(n)] => Some(i64::from(*n)),
        [Token::Delim('-'), Token::Number(n)] => Some(-i64::from(*n)),
        _ if looks_like_calc(value) => {
            let v = number_math(parse_calc(value)?)?;
            // `as` saturates at the `i64` range.
            Some((v + 0.5).floor() as i64)
        }
        _ => None,
    }
}

/// Parse a value as a `<percentage>`: a literal, or a math function of
/// percentages alone (`calc(10% + 5%)`, [`CalcKind::Percent`]). Returns
/// the percentage (50% is 50). Used to validate registered
/// `<percentage>` values.
pub(crate) fn percentage(value: &[Token]) -> Option<f64> {
    match value {
        [Token::Percentage(p)] => Some(*p),
        [Token::Delim('-'), Token::Percentage(p)] => Some(-*p),
        _ if looks_like_calc(value) => {
            let expr = parse_calc(value)?;
            if expr.kind() != Some(CalcKind::Percent) || expr.needs_context() {
                return None;
            }
            // Against a basis of 100 a percentage is itself.
            let v = expr.resolve_f64(&ResolveCtx::new(100));
            Some(if v.is_nan() { 0.0 } else { v })
        }
        _ => None,
    }
}

/// The largest angle magnitude, in degrees: an infinite angle clamps to
/// it (CSS Values 4 §10.9). `f32::MAX`, the widest an angle is stored.
pub const MAX_ANGLE_DEGREES: f64 = f32::MAX as f64;

/// Parse a value as an `<angle>` (CSS Values 4 §7.1): a dimension in
/// `deg`, `grad`, `rad` or `turn`, or a math function of type `<angle>`
/// (`calc(1turn - 90deg)`, `atan2(1, 1)`). Returns degrees, the unit the
/// color hues take, always finite: NaN is 0 and an infinity clamps to
/// [`MAX_ANGLE_DEGREES`] (§10.9). A math function holding a percentage
/// or a viewport unit is no angle here (`atan2(50%, 10)`: nothing to
/// resolve either against). A bare number is no angle either — a hue
/// grammar that takes one parses it itself.
pub fn parse_angle(value: &[Token]) -> Option<f64> {
    let (negative, rest) = match value {
        [Token::Delim('-'), rest @ ..] => (true, rest),
        _ => (false, value),
    };
    let radians = match rest {
        [Token::Dimension { value, unit, .. }] => {
            let unit = CalcUnit::parse(unit).filter(|u| u.kind() == CalcKind::Angle)?;
            CalcExpr::Dimension {
                value: *value,
                unit,
            }
            .resolve_f64(&ResolveCtx::new(0))
        }
        _ if !negative && looks_like_calc(rest) => {
            let expr = parse_calc(rest)?;
            if expr.kind() != Some(CalcKind::Angle)
                || expr.contains_percent()
                || expr.needs_context()
            {
                return None;
            }
            expr.resolve_f64(&ResolveCtx::new(0))
        }
        _ => return None,
    };
    let degrees = radians.to_degrees();
    let degrees = if degrees.is_nan() {
        0.0
    } else {
        degrees.clamp(-MAX_ANGLE_DEGREES, MAX_ANGLE_DEGREES)
    };
    Some(if negative { -degrees } else { degrees })
}

/// Split a declaration value into its component values (CSS Syntax 3
/// §5.4.9): a function runs to its matching `)`, a `-` delimiter
/// belongs to the numeric token after it, and every other token
/// stands alone. `None` on an unbalanced function.
pub(crate) fn components(value: &[Token]) -> Option<Vec<&[Token]>> {
    let mut out = Vec::with_capacity(4);
    let mut i = 0;
    while i < value.len() {
        let end = match &value[i] {
            Token::Function(_) | Token::LParen => matching_paren(value, i)? + 1,
            Token::Delim('-')
                if matches!(
                    value.get(i + 1),
                    Some(Token::Number(_) | Token::Float(_) | Token::Percentage(_))
                        | Some(Token::Dimension { .. })
                ) =>
            {
                i + 2
            }
            _ => i + 1,
        };
        out.push(&value[i..end]);
        i = end;
    }
    Some(out)
}

/// Split a value at its top-level commas (CSS Values 4 `#`
/// multiplier). `None` when a segment is empty.
pub(crate) fn split_commas(value: &[Token]) -> Option<Vec<&[Token]>> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, t) in value.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.checked_sub(1)?,
            Token::Comma if depth == 0 => {
                out.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out.iter().all(|s| !s.is_empty()).then_some(out)
}

/// Index of the `)` closing the function or `(` at `open`.
fn matching_paren(value: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (j, t) in value.iter().enumerate().skip(open) {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
#[path = "numeric_tests.rs"]
mod tests;
