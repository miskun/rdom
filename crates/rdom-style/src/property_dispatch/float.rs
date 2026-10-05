//! `float` and `clear` (CSS 2.1 §9.5.1 / §9.5.2, CSS Logical 1 §2.3):
//! their `set` and `serialize` arms.

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{parse_clear, parse_float};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "float" => parse_float(value).map(|f| {
            style.float = Some(Value::Specified(f));
        }),
        "clear" => parse_clear(value).map(|c| {
            style.clear = Some(Value::Specified(c));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "float" => style
            .float
            .as_ref()
            .and_then(specified)
            .map(|f| f.keyword().to_string()),
        "clear" => style
            .clear
            .as_ref()
            .and_then(specified)
            .map(|c| c.keyword().to_string()),
        _ => return None,
    })
}
