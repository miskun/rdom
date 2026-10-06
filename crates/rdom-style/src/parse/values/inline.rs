//! CSS Inline Layout 3 values: `line-height`.

use super::numeric::{LengthPercentage, Range, length_percentage, number};
use crate::layout::LineHeight;
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
