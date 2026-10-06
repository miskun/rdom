//! CSS Inline Layout 3 values: `line-height`, `vertical-align`.

use super::numeric::{LengthPercentage, Range, length_percentage, number};
use crate::layout::{LineHeight, VerticalAlign};
use crate::parse::token::Token;
use crate::property_dispatch::serialize_math;

/// `line-height: normal | <number [0,∞]> | <length-percentage [0,∞]>`
/// (CSS Inline 3 §5.1). A bare number is the `<number>` (rdom's unitless
/// cell would be the same rows); a length known at parse time is rows, a
/// percentage or a length in a context unit (`lh`, `rlh`, a viewport
/// unit) is kept for the cascade to compute.
pub fn parse_line_height(value: &[Token]) -> Option<LineHeight> {
    if matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case("normal")) {
        return Some(LineHeight::Normal);
    }
    if let Some(n) = number(value, Range::NonNegative) {
        return n.is_finite().then_some(LineHeight::Number(n as f32));
    }
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => Some(LineHeight::Rows(n as f32)),
        LengthPercentage::Cells(v) => v.is_finite().then_some(LineHeight::Rows(v as f32)),
        LengthPercentage::Expr(e) => Some(LineHeight::Calc(Box::new(e))),
    }
}

/// `line-height`'s serialization (CSSOM §6.7.2): the keyword, the number,
/// a parse-time length in `ch` (rdom's cell), or the expression.
pub fn serialize_line_height(value: &LineHeight) -> String {
    match value {
        LineHeight::Normal => "normal".to_string(),
        LineHeight::Number(n) => format!("{n}"),
        LineHeight::Rows(n) => format!("{n}ch"),
        LineHeight::Calc(expr) => serialize_math(expr),
    }
}

/// `vertical-align: baseline | sub | super | text-top | text-bottom |
/// middle | top | bottom | <length-percentage>` (CSS 2.1 §10.8.1), a
/// length of either sign (positive raises); a bare number is rdom's cell.
pub fn parse_vertical_align(value: &[Token]) -> Option<VerticalAlign> {
    const KEYWORDS: [VerticalAlign; 8] = [
        VerticalAlign::Baseline,
        VerticalAlign::Sub,
        VerticalAlign::Super,
        VerticalAlign::TextTop,
        VerticalAlign::TextBottom,
        VerticalAlign::Middle,
        VerticalAlign::Top,
        VerticalAlign::Bottom,
    ];
    if let [Token::Ident(s)] = value {
        return KEYWORDS
            .into_iter()
            .find(|k| k.keyword().is_some_and(|w| s.eq_ignore_ascii_case(w)));
    }
    match length_percentage(value, Range::Any)? {
        LengthPercentage::Integer(n) => Some(VerticalAlign::Rows(n as f32)),
        LengthPercentage::Cells(v) => v.is_finite().then_some(VerticalAlign::Rows(v as f32)),
        LengthPercentage::Expr(e) => Some(VerticalAlign::Calc(Box::new(e))),
    }
}

/// `vertical-align`'s serialization: the keyword, rows as rdom's cell, or
/// the expression.
pub fn serialize_vertical_align(value: &VerticalAlign) -> String {
    match value {
        VerticalAlign::Rows(n) => format!("{n}"),
        VerticalAlign::Calc(expr) => serialize_math(expr),
        keyword => keyword.keyword().unwrap_or_default().to_string(),
    }
}
