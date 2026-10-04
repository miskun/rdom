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

use super::calc::{looks_like_calc, parse_calc};
use crate::calc::{CalcExpr, ResolveCtx};
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
    /// A length known at parse time, in (possibly fractional) cells —
    /// a math function without a percentage. Rounded onto the grid
    /// where it becomes a property value.
    Cells(f64),
    /// A value that needs the layout's basis: a percentage, alone or
    /// inside a math function.
    Expr(CalcExpr),
}

/// `v` cells rounded onto the grid (ties to even, as `calc()` rounds)
/// and clamped to `0..=u16::MAX`.
pub(crate) fn cells_u16(v: f64) -> u16 {
    crate::calc::round_half_to_even(v).clamp(0, i32::from(u16::MAX)) as u16
}

/// `v` cells rounded onto the grid, as a signed cell count.
pub(crate) fn cells_i32(v: f64) -> i32 {
    crate::calc::round_half_to_even(v)
}

/// Parse one component value as a `<length-percentage>`: a bare
/// integer (cells), a percentage, or a math function. A leading `-`
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
        [Token::Percentage(p)] => Some(LengthPercentage::Expr(CalcExpr::Percent(sign * *p))),
        _ if !negative && looks_like_calc(rest) => {
            let expr = parse_calc(rest)?;
            Some(if expr.contains_percent() {
                LengthPercentage::Expr(expr)
            } else {
                LengthPercentage::Cells(expr.resolve_f64(&ResolveCtx::new(0)))
            })
        }
        _ => None,
    }
}

/// Parse one component value as a `<number>` (CSS Values 4 §5.4):
/// an integer or a fraction (`0.5`, `.5`, `1e3`). A leading `-` is
/// the literal's sign.
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
        _ => return None,
    };
    Some(if negative { -n } else { n })
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
