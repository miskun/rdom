//! Box spacing: `gap`, the `padding` shorthand / longhands and the
//! `margin` shorthand / longhands, each accepting `calc()` with
//! percent-bearing forms kept symbolic for layout.

use super::calc::{looks_like_calc, parse_calc};
use crate::calc::CalcExpr;
use crate::layout::Padding;
use crate::parse::token::Token;
use crate::{TuiStyle, Value};

/// `gap`: `<length-percentage>` — whole cells, a bare percentage or a
/// `calc()`; percent-bearing forms stay symbolic until layout knows
/// the container size.
pub fn parse_gap(value: &[Token]) -> Option<crate::layout::GapValue> {
    use crate::layout::GapValue;
    match value {
        [Token::Number(n)] => u16::try_from(*n).ok().map(GapValue::Cells),
        [Token::Percentage(p)] if *p >= 0.0 => {
            Some(GapValue::Calc(Box::new(CalcExpr::Percent(*p))))
        }
        _ if looks_like_calc(value) => {
            let expr = parse_calc(value)?;
            if expr.contains_percent() {
                Some(GapValue::Calc(Box::new(expr)))
            } else {
                let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
                Some(GapValue::Cells(cells.clamp(0, i32::from(u16::MAX)) as u16))
            }
        }
        _ => None,
    }
}

/// Padding shorthand expansion. Accepts 1..=4 unsigned integers.
/// Order matches CSS: top, right, bottom, left (clockwise from top).
pub fn parse_padding_shorthand(value: &[Token]) -> Option<Padding> {
    // Split on `calc(...)` function boundaries + bare numbers.
    // Each value-position must be either a bare unsigned number
    // OR a `calc(...)` that resolves to a constant integer (no
    // percent — padding/margin's u16 field type can't carry a
    // Calc AST; see DIVERGENCES.md).
    let nums = split_padding_values(value)?;
    let p = match nums.as_slice() {
        [a] => Padding {
            top: a.clone(),
            right: a.clone(),
            bottom: a.clone(),
            left: a.clone(),
        },
        [a, b] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: a.clone(),
            left: b.clone(),
        },
        [a, b, c] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: c.clone(),
            left: b.clone(),
        },
        [a, b, c, d] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: c.clone(),
            left: d.clone(),
        },
        _ => return None,
    };
    Some(p)
}

/// Split a value-token slice into 1..=4 padding values. Each
/// position accepts:
/// - bare `Token::Number(n)` with `n >= 0` → `PaddingValue::Cells(n)`.
/// - `calc(<expr>)` — constant calcs resolve immediately to
///   `Cells`; percent-bearing calcs are preserved as
///   `PaddingValue::Calc` for layout-time resolution against the
///   containing-block width (closes `CALC-PADMARG-1`).
fn split_padding_values(value: &[Token]) -> Option<Vec<crate::layout::PaddingValue>> {
    use crate::layout::PaddingValue;
    let mut vals = Vec::with_capacity(4);
    let mut i = 0;
    while i < value.len() {
        match &value[i] {
            Token::Number(n) if *n >= 0 => {
                vals.push(PaddingValue::Cells(u16::try_from(*n).ok()?));
                i += 1;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("calc") => {
                // Scan forward for the matching `)`.
                let mut depth = 1usize;
                let mut j = i + 1;
                while j < value.len() && depth > 0 {
                    match &value[j] {
                        Token::LParen | Token::Function(_) => depth += 1,
                        Token::RParen => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth != 0 {
                    return None;
                }
                let expr = parse_calc(&value[i..j])?;
                if expr.contains_percent() {
                    // Keep the AST around — layout resolves against
                    // containing-block width per CSS 2.1 §8.4.
                    vals.push(PaddingValue::Calc(Box::new(expr)));
                } else {
                    let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
                    vals.push(PaddingValue::Cells(cells.max(0).min(u16::MAX as i32) as u16));
                }
                i = j;
            }
            _ => return None,
        }
        if vals.len() > 4 {
            return None;
        }
    }
    if vals.is_empty() {
        return None;
    }
    Some(vals)
}

/// Read the current padding from `style`, defaulting to all-zero
/// when nothing is set or when the existing value is `Inherit` /
/// `Initial`. Used by the per-side longhands so consecutive
/// declarations combine instead of overwriting.
/// Parse a single padding value (used by `padding-*` longhands).
/// Accepts `<number>` (cells) or `calc(<expr>)` — constant calc
/// resolves at parse time; percent-bearing calc is preserved as
/// `PaddingValue::Calc` for layout-time resolution.
pub fn parse_padding_value(value: &[Token]) -> Option<crate::layout::PaddingValue> {
    let vals = split_padding_values(value)?;
    if vals.len() == 1 {
        Some(vals.into_iter().next().unwrap())
    } else {
        None
    }
}

pub fn current_padding(style: &TuiStyle) -> Padding {
    match &style.padding {
        Some(Value::Specified(p)) => p.clone(),
        _ => Padding::default(),
    }
}

/// Parse one margin token-group: either `auto` or a signed integer
/// (positive `Number`, or `Delim('-')` followed by `Number`).
/// Returns the value and the number of tokens consumed.
fn parse_margin_value_at(
    value: &[Token],
    start: usize,
) -> Option<(crate::layout::MarginValue, usize)> {
    use crate::layout::MarginValue;
    match value.get(start) {
        Some(Token::Ident(s)) if s.eq_ignore_ascii_case("auto") => Some((MarginValue::Auto, 1)),
        Some(Token::Number(n)) => {
            if *n > i16::MAX as i32 || *n < i16::MIN as i32 {
                return None;
            }
            Some((MarginValue::Cells(*n as i16), 1))
        }
        Some(Token::Delim('-')) => match value.get(start + 1) {
            Some(Token::Number(n)) if -(*n) >= i16::MIN as i32 => {
                Some((MarginValue::Cells(-(*n) as i16), 2))
            }
            _ => None,
        },
        // `calc(<expr>)` — constant resolves immediately; percent-
        // bearing preserved as `MarginValue::Calc` for layout-time
        // resolution against containing-block width.
        Some(Token::Function(name)) if name.eq_ignore_ascii_case("calc") => {
            let mut depth = 1usize;
            let mut j = start + 1;
            while j < value.len() && depth > 0 {
                match &value[j] {
                    Token::LParen | Token::Function(_) => depth += 1,
                    Token::RParen => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            let expr = parse_calc(&value[start..j])?;
            let consumed = j - start;
            if expr.contains_percent() {
                Some((MarginValue::Calc(Box::new(expr)), consumed))
            } else {
                let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
                if !(i16::MIN as i32..=i16::MAX as i32).contains(&cells) {
                    return None;
                }
                Some((MarginValue::Cells(cells as i16), consumed))
            }
        }
        _ => None,
    }
}

/// `margin: <v>` | `<v> <v>` | `<v> <v> <v>` | `<v> <v> <v> <v>`
/// where each `<v>` is `auto` or a signed integer. CSS expansion:
/// - 1 value → all four sides
/// - 2 values → top/bottom = a, right/left = b
/// - 3 values → top = a, right/left = b, bottom = c
/// - 4 values → top, right, bottom, left
pub fn parse_margin_shorthand(value: &[Token]) -> Option<crate::layout::Margin> {
    use crate::layout::{Margin, MarginValue};
    let mut vals: Vec<MarginValue> = Vec::with_capacity(4);
    let mut i = 0;
    while i < value.len() {
        let (v, consumed) = parse_margin_value_at(value, i)?;
        vals.push(v);
        i += consumed;
        if vals.len() > 4 {
            return None;
        }
    }
    let m = match vals.as_slice() {
        [a] => Margin::new(a.clone(), a.clone(), a.clone(), a.clone()),
        [a, b] => Margin::new(a.clone(), b.clone(), a.clone(), b.clone()),
        [a, b, c] => Margin::new(a.clone(), b.clone(), c.clone(), b.clone()),
        [a, b, c, d] => Margin::new(a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return None,
    };
    Some(m)
}

/// Parse a single margin longhand (`margin-top`, etc.).
pub fn parse_margin_longhand(value: &[Token]) -> Option<crate::layout::MarginValue> {
    let (v, consumed) = parse_margin_value_at(value, 0)?;
    if consumed != value.len() {
        return None;
    }
    Some(v)
}

/// Read the current margin from `style`, defaulting to all-zero when
/// nothing is set. Used by per-side longhands so consecutive
/// declarations combine instead of overwriting.
pub fn current_margin(style: &TuiStyle) -> crate::layout::Margin {
    match &style.margin {
        Some(Value::Specified(m)) => m.clone(),
        _ => crate::layout::Margin::default(),
    }
}
