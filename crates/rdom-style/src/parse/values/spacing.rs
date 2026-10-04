//! Box spacing: `gap`, the `padding` shorthand / longhands and the
//! `margin` shorthand / longhands, each accepting `calc()` with
//! percent-bearing forms kept symbolic for layout.

use super::numeric::{
    LengthPercentage, Range, cells_i32, cells_u16, components, length_percentage,
};
use crate::layout::{GapValue, MarginValue, Padding, PaddingValue};
use crate::parse::token::Token;
use crate::{TuiStyle, Value};

/// `gap`: `<length-percentage [0,∞]>` — whole cells, a percentage or a
/// math function; percent-bearing forms stay symbolic until layout
/// knows the container size.
pub fn parse_gap(value: &[Token]) -> Option<GapValue> {
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(GapValue::Cells),
        LengthPercentage::Cells(v) => Some(GapValue::Cells(cells_u16(v))),
        LengthPercentage::Expr(e) => Some(GapValue::Calc(Box::new(e))),
    }
}

/// Expand 1..=4 side values clockwise from the top (CSS Box 3 §3.2 /
/// §4.2): one → all, two → vertical / horizontal, three → top /
/// horizontal / bottom, four → top, right, bottom, left.
fn expand_sides<T: Clone>(vals: &[T]) -> Option<[T; 4]> {
    Some(match vals {
        [a] => [a.clone(), a.clone(), a.clone(), a.clone()],
        [a, b] => [a.clone(), b.clone(), a.clone(), b.clone()],
        [a, b, c] => [a.clone(), b.clone(), c.clone(), b.clone()],
        [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
        _ => return None,
    })
}

/// Padding shorthand: 1..=4 `<length-percentage [0,∞]>` values, top,
/// right, bottom, left (clockwise from top).
pub fn parse_padding_shorthand(value: &[Token]) -> Option<Padding> {
    let vals = components(value)?
        .into_iter()
        .map(parse_padding_value)
        .collect::<Option<Vec<_>>>()?;
    let [top, right, bottom, left] = expand_sides(&vals)?;
    Some(Padding {
        top,
        right,
        bottom,
        left,
    })
}

/// One padding value (the `padding-*` longhands):
/// `<length-percentage [0,∞]>`. A percentage — alone or in a math
/// function — stays symbolic for layout, which resolves it against the
/// containing block's width on every side (CSS Box 3 §4.2).
pub fn parse_padding_value(value: &[Token]) -> Option<PaddingValue> {
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(PaddingValue::Cells),
        LengthPercentage::Cells(v) => Some(PaddingValue::Cells(cells_u16(v))),
        LengthPercentage::Expr(e) => Some(PaddingValue::Calc(Box::new(e))),
    }
}

/// Read the current padding from `style`, defaulting to all-zero
/// when nothing is set. Used by the per-side longhands so consecutive
/// declarations combine instead of overwriting.
pub fn current_padding(style: &TuiStyle) -> Padding {
    match &style.padding {
        Some(Value::Specified(p)) => p.clone(),
        _ => Padding::default(),
    }
}

/// One margin value: `auto` | `<length-percentage>` (either sign). A
/// percentage stays symbolic for layout, which resolves it against the
/// containing block's width on every side (CSS Box 3 §3.2).
fn parse_margin_value(value: &[Token]) -> Option<MarginValue> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(MarginValue::Auto),
        _ => match length_percentage(value, Range::Any)? {
            LengthPercentage::Integer(n) => i16::try_from(n).ok().map(MarginValue::Cells),
            LengthPercentage::Cells(v) => i16::try_from(cells_i32(v)).ok().map(MarginValue::Cells),
            LengthPercentage::Expr(e) => Some(MarginValue::Calc(Box::new(e))),
        },
    }
}

/// `margin`: 1..=4 margin values, expanded clockwise from the top.
pub fn parse_margin_shorthand(value: &[Token]) -> Option<crate::layout::Margin> {
    let vals = components(value)?
        .into_iter()
        .map(parse_margin_value)
        .collect::<Option<Vec<_>>>()?;
    let [top, right, bottom, left] = expand_sides(&vals)?;
    Some(crate::layout::Margin::new(top, right, bottom, left))
}

/// Parse a single margin longhand (`margin-top`, etc.).
pub fn parse_margin_longhand(value: &[Token]) -> Option<MarginValue> {
    parse_margin_value(value)
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
