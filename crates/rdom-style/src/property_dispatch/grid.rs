//! The grid properties (CSS Grid Layout 2): their `set` and `serialize`
//! arms — the track lists of `grid-template-columns` / `-rows` (§7.2)
//! and the implicit track sizes of `grid-auto-columns` / `-rows` (§7.6).

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{
    parse_grid_template, parse_track_sizes, serialize_grid_template, serialize_track_sizes,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the grid properties. `None` when `name` is
/// not one; `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "grid-template-columns" => parse_grid_template(value).map(|t| {
            style.grid_template_columns = Some(Value::Specified(t));
        }),
        "grid-template-rows" => parse_grid_template(value).map(|t| {
            style.grid_template_rows = Some(Value::Specified(t));
        }),
        "grid-auto-columns" => parse_track_sizes(value).map(|t| {
            style.grid_auto_columns = Some(Value::Specified(t));
        }),
        "grid-auto-rows" => parse_track_sizes(value).map(|t| {
            style.grid_auto_rows = Some(Value::Specified(t));
        }),
        _ => return None,
    })
}

/// Serialize one of the grid properties. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "grid-template-columns" => style
            .grid_template_columns
            .as_ref()
            .and_then(specified)
            .map(serialize_grid_template),
        "grid-template-rows" => style
            .grid_template_rows
            .as_ref()
            .and_then(specified)
            .map(serialize_grid_template),
        "grid-auto-columns" => style
            .grid_auto_columns
            .as_ref()
            .and_then(specified)
            .map(|t| serialize_track_sizes(t)),
        "grid-auto-rows" => style
            .grid_auto_rows
            .as_ref()
            .and_then(specified)
            .map(|t| serialize_track_sizes(t)),
        _ => return None,
    })
}
