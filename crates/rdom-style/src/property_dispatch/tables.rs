//! The table properties (CSS 2.1 §17): `table-layout` (§17.5.2) and
//! `caption-side` (§17.4.1), `empty-cells` (§17.6.1.1) — their `set` and
//! `serialize` arms.
//! (`border-collapse` / `border-spacing` are `set.rs`'s / `border.rs`'s.)

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{parse_caption_side, parse_empty_cells, parse_table_layout};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let table = &mut style.table;
    Some(match name {
        "table-layout" => parse_table_layout(value).map(|v| {
            table.table_layout = Some(Value::Specified(v));
        }),
        "caption-side" => parse_caption_side(value).map(|v| {
            table.caption_side = Some(Value::Specified(v));
        }),
        "empty-cells" => parse_empty_cells(value).map(|v| {
            table.empty_cells = Some(Value::Specified(v));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let table = &style.table;
    Some(match name {
        "table-layout" => table
            .table_layout
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "caption-side" => table
            .caption_side
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "empty-cells" => table
            .empty_cells
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        _ => return None,
    })
}
