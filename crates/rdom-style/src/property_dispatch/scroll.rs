//! The scrolling properties — `overscroll-behavior` and its longhands
//! (CSS Overscroll Behavior 1 §3): their `set` and `serialize` arms.
//! The flow-relative longhands are `logical`'s block-axis aliases.

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{parse_overscroll_behavior, parse_overscroll_behavior_shorthand};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "overscroll-behavior" => parse_overscroll_behavior_shorthand(value).map(|(x, y)| {
            style.overscroll_behavior_x = Some(Value::Specified(x));
            style.overscroll_behavior_y = Some(Value::Specified(y));
        }),
        "overscroll-behavior-x" => parse_overscroll_behavior(value).map(|b| {
            style.overscroll_behavior_x = Some(Value::Specified(b));
        }),
        "overscroll-behavior-y" => parse_overscroll_behavior(value).map(|b| {
            style.overscroll_behavior_y = Some(Value::Specified(b));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let x = style.overscroll_behavior_x.as_ref().and_then(specified);
    let y = style.overscroll_behavior_y.as_ref().and_then(specified);
    Some(match name {
        "overscroll-behavior-x" => x.map(|b| b.keyword().to_string()),
        "overscroll-behavior-y" => y.map(|b| b.keyword().to_string()),
        // One value when the axes match, else `x y`.
        "overscroll-behavior" => match (x, y) {
            (Some(x), Some(y)) if x == y => Some(x.keyword().to_string()),
            (Some(x), Some(y)) => Some(format!("{} {}", x.keyword(), y.keyword())),
            _ => None,
        },
        _ => return None,
    })
}
