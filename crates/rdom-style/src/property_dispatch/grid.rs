//! The grid properties (CSS Grid Layout 2): their `set` and `serialize`
//! arms — the track lists of `grid-template-columns` / `-rows` (§7.2),
//! the implicit track sizes of `grid-auto-columns` / `-rows` (§7.6),
//! `grid-auto-flow` (§7.7), and the placement longhands and shorthands
//! (§8.3, §8.4).

use super::value_serializers::specified;
use crate::layout::GridLine;
use crate::parse::token::Token;
use crate::parse::values::{
    parse_grid_area, parse_grid_auto_flow, parse_grid_line, parse_grid_line_pair,
    parse_grid_template, parse_track_sizes, serialize_grid_area, serialize_grid_auto_flow,
    serialize_grid_line, serialize_grid_line_pair, serialize_grid_template, serialize_track_sizes,
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
        "grid-auto-flow" => parse_grid_auto_flow(value).map(|f| {
            style.grid_auto_flow = Some(Value::Specified(f));
        }),
        "grid-row-start" => parse_grid_line(value).map(|l| {
            style.grid_row_start = Some(Value::Specified(l));
        }),
        "grid-row-end" => parse_grid_line(value).map(|l| {
            style.grid_row_end = Some(Value::Specified(l));
        }),
        "grid-column-start" => parse_grid_line(value).map(|l| {
            style.grid_column_start = Some(Value::Specified(l));
        }),
        "grid-column-end" => parse_grid_line(value).map(|l| {
            style.grid_column_end = Some(Value::Specified(l));
        }),
        "grid-row" => parse_grid_line_pair(value).map(|(start, end)| {
            style.grid_row_start = Some(Value::Specified(start));
            style.grid_row_end = Some(Value::Specified(end));
        }),
        "grid-column" => parse_grid_line_pair(value).map(|(start, end)| {
            style.grid_column_start = Some(Value::Specified(start));
            style.grid_column_end = Some(Value::Specified(end));
        }),
        "grid-area" => parse_grid_area(value).map(|[rs, cs, re, ce]| {
            style.grid_row_start = Some(Value::Specified(rs));
            style.grid_column_start = Some(Value::Specified(cs));
            style.grid_row_end = Some(Value::Specified(re));
            style.grid_column_end = Some(Value::Specified(ce));
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
        "grid-auto-flow" => style
            .grid_auto_flow
            .as_ref()
            .and_then(specified)
            .map(|f| serialize_grid_auto_flow(*f)),
        "grid-row-start" => line(&style.grid_row_start).map(serialize_grid_line),
        "grid-row-end" => line(&style.grid_row_end).map(serialize_grid_line),
        "grid-column-start" => line(&style.grid_column_start).map(serialize_grid_line),
        "grid-column-end" => line(&style.grid_column_end).map(serialize_grid_line),
        // A shorthand serializes when all its longhands are set (CSSOM
        // §6.7.2).
        "grid-row" => line(&style.grid_row_start)
            .zip(line(&style.grid_row_end))
            .map(|(s, e)| serialize_grid_line_pair(s, e)),
        "grid-column" => line(&style.grid_column_start)
            .zip(line(&style.grid_column_end))
            .map(|(s, e)| serialize_grid_line_pair(s, e)),
        "grid-area" => match (
            line(&style.grid_row_start),
            line(&style.grid_column_start),
            line(&style.grid_row_end),
            line(&style.grid_column_end),
        ) {
            (Some(rs), Some(cs), Some(re), Some(ce)) => Some(serialize_grid_area(&[
                rs.clone(),
                cs.clone(),
                re.clone(),
                ce.clone(),
            ])),
            _ => None,
        },
        _ => return None,
    })
}

/// The specified value of a placement longhand.
fn line(field: &Option<Value<GridLine>>) -> Option<&GridLine> {
    field.as_ref().and_then(specified)
}
