//! The CSS Inline Layout 3 properties: `line-height` and
//! `vertical-align` — their `set` and `serialize` arms.

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{
    parse_line_height, parse_vertical_align, serialize_line_height, serialize_vertical_align,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "line-height" => parse_line_height(value).map(|l| {
            style.text.line_height = Some(Value::Specified(l));
        }),
        "vertical-align" => parse_vertical_align(value).map(|v| {
            style.vertical_align = Some(Value::Specified(v));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "line-height" => style
            .text
            .line_height
            .as_ref()
            .and_then(specified)
            .map(serialize_line_height),
        "vertical-align" => style
            .vertical_align
            .as_ref()
            .and_then(specified)
            .map(serialize_vertical_align),
        _ => return None,
    })
}
