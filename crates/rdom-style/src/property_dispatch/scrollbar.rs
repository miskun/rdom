//! `scrollbar-gutter` (CSS Overflow 3 §3.3), `scrollbar-width` and
//! `scrollbar-color` (CSS Scrollbars 1 §2–§3): their `set` and
//! `serialize` arms.

use super::value_serializers::{serialize_color, specified};
use crate::layout::ScrollbarColor;
use crate::parse::token::Token;
use crate::parse::values::{parse_scrollbar_color, parse_scrollbar_gutter, parse_scrollbar_width};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "scrollbar-gutter" => parse_scrollbar_gutter(value).map(|g| {
            style.scrollbar_gutter = Some(Value::Specified(g));
        }),
        "scrollbar-width" => parse_scrollbar_width(value).map(|w| {
            style.scrollbar_width = Some(Value::Specified(w));
        }),
        "scrollbar-color" => parse_scrollbar_color(value).map(|c| {
            style.scrollbar_color = Some(Value::Specified(c));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "scrollbar-gutter" => style
            .scrollbar_gutter
            .as_ref()
            .and_then(specified)
            .map(|g| g.keyword().to_string()),
        "scrollbar-width" => style
            .scrollbar_width
            .as_ref()
            .and_then(specified)
            .map(|w| w.keyword().to_string()),
        "scrollbar-color" => style
            .scrollbar_color
            .as_ref()
            .and_then(specified)
            .map(|c| match c {
                ScrollbarColor::Auto => "auto".to_string(),
                ScrollbarColor::Colors { thumb, track } => {
                    format!("{} {}", serialize_color(thumb), serialize_color(track))
                }
            }),
        _ => return None,
    })
}
