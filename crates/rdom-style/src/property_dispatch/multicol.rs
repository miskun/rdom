//! The multi-column properties (CSS Multi-column 1: `columns`,
//! `column-count`, `column-width`, `column-rule` and its longhands,
//! `column-span`, `column-fill`) and the fragmentation properties (CSS
//! Fragmentation 3: `break-*`, the legacy `page-break-*`, `orphans`,
//! `widows`, `box-decoration-break`) — their `set` and `serialize` arms.
//! (`column-gap` is `set.rs`'s, shared with flex and grid.)

use super::border::serialize_line_width;
use super::value_serializers::{border_style_keyword, serialize_color, serialize_math, specified};
use crate::layout::{BorderStyle, BorderWidth, ColumnCount, ColumnWidth};
use crate::parse::token::Token;
use crate::parse::values::{
    page_break_between_keyword, parse_box_decoration_break, parse_break_between,
    parse_break_inside, parse_column_count, parse_column_fill, parse_column_rule,
    parse_column_rule_color, parse_column_rule_style, parse_column_rule_width, parse_column_span,
    parse_column_width, parse_columns, parse_orphans_widows, parse_page_break_between,
    parse_page_break_inside,
};
use crate::{TuiColor, TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let m = &mut style.multicol;
    let f = &mut style.fragmentation;
    Some(match name {
        "column-count" => {
            parse_column_count(value).map(|v| m.column_count = Some(Value::Specified(v)))
        }
        "column-width" => {
            parse_column_width(value).map(|v| m.column_width = Some(Value::Specified(v)))
        }
        "columns" => parse_columns(value).map(|(width, count)| {
            m.column_width = Some(Value::Specified(width));
            m.column_count = Some(Value::Specified(count));
        }),
        "column-rule-style" => {
            parse_column_rule_style(value).map(|v| m.column_rule_style = Some(Value::Specified(v)))
        }
        "column-rule-width" => {
            parse_column_rule_width(value).map(|v| m.column_rule_width = Some(Value::Specified(v)))
        }
        "column-rule-color" => {
            parse_column_rule_color(value).map(|v| m.column_rule_color = Some(Value::Specified(v)))
        }
        "column-rule" => parse_column_rule(value).map(|(width, s, color)| {
            m.column_rule_width = Some(Value::Specified(width));
            m.column_rule_style = Some(Value::Specified(s));
            m.column_rule_color = Some(Value::Specified(color));
        }),
        "column-span" => {
            parse_column_span(value).map(|v| m.column_span = Some(Value::Specified(v)))
        }
        "column-fill" => {
            parse_column_fill(value).map(|v| m.column_fill = Some(Value::Specified(v)))
        }
        "break-before" => {
            parse_break_between(value).map(|v| f.break_before = Some(Value::Specified(v)))
        }
        "break-after" => {
            parse_break_between(value).map(|v| f.break_after = Some(Value::Specified(v)))
        }
        "break-inside" => {
            parse_break_inside(value).map(|v| f.break_inside = Some(Value::Specified(v)))
        }
        "page-break-before" => {
            parse_page_break_between(value).map(|v| f.break_before = Some(Value::Specified(v)))
        }
        "page-break-after" => {
            parse_page_break_between(value).map(|v| f.break_after = Some(Value::Specified(v)))
        }
        "page-break-inside" => {
            parse_page_break_inside(value).map(|v| f.break_inside = Some(Value::Specified(v)))
        }
        "orphans" => parse_orphans_widows(value).map(|v| f.orphans = Some(Value::Specified(v))),
        "widows" => parse_orphans_widows(value).map(|v| f.widows = Some(Value::Specified(v))),
        "box-decoration-break" => parse_box_decoration_break(value)
            .map(|v| f.box_decoration_break = Some(Value::Specified(v))),
        _ => return None,
    })
}

fn count_text(c: &ColumnCount) -> String {
    match c {
        ColumnCount::Auto => "auto".to_string(),
        ColumnCount::Count(n) => n.to_string(),
    }
}

fn width_text(w: &ColumnWidth) -> String {
    match w {
        ColumnWidth::Auto => "auto".to_string(),
        ColumnWidth::Cells(n) => n.to_string(),
        ColumnWidth::Calc(e) => serialize_math(e),
    }
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let m = &style.multicol;
    let f = &style.fragmentation;
    let count = m.column_count.as_ref().and_then(specified);
    let width = m.column_width.as_ref().and_then(specified);
    let rule_width = m.column_rule_width.as_ref().and_then(specified);
    let rule_style = m.column_rule_style.as_ref().and_then(specified);
    let rule_color = m.column_rule_color.as_ref().and_then(specified);
    Some(match name {
        "column-count" => count.map(count_text),
        "column-width" => width.map(width_text),
        // The shortest form (CSSOM §6.7.2): the components off `auto`, a
        // width before a count; `auto` when both are.
        "columns" => match (width, count) {
            (Some(w), Some(c)) => {
                let mut parts = Vec::new();
                // A width in `ch` (one cell each), so it reads back as a
                // width, not a count.
                match w {
                    ColumnWidth::Auto => {}
                    ColumnWidth::Cells(n) => parts.push(format!("{n}ch")),
                    ColumnWidth::Calc(_) => parts.push(width_text(w)),
                }
                if *c != ColumnCount::Auto {
                    parts.push(count_text(c));
                }
                Some(if parts.is_empty() {
                    "auto".to_string()
                } else {
                    parts.join(" ")
                })
            }
            _ => None,
        },
        "column-rule-style" => rule_style.map(|s| border_style_keyword(*s).to_string()),
        "column-rule-width" => rule_width.map(serialize_line_width),
        "column-rule-color" => rule_color.map(serialize_color),
        // The components off their initial values — width, style, color —
        // `medium` when all are (CSSOM's shortest form keeps one).
        "column-rule" => match (rule_width, rule_style, rule_color) {
            (Some(w), Some(s), Some(c)) => {
                let mut parts = Vec::new();
                if *w != BorderWidth::Medium {
                    parts.push(serialize_line_width(w));
                }
                if *s != BorderStyle::None {
                    parts.push(border_style_keyword(*s).to_string());
                }
                if *c != TuiColor::CurrentColor {
                    parts.push(serialize_color(c));
                }
                Some(if parts.is_empty() {
                    "medium".to_string()
                } else {
                    parts.join(" ")
                })
            }
            _ => None,
        },
        "column-span" => m
            .column_span
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "column-fill" => m
            .column_fill
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "break-before" => f
            .break_before
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "break-after" => f
            .break_after
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        "break-inside" => f
            .break_inside
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        // §3.4: a legacy shorthand reads its longhand back where it can
        // spell it, and is empty where it cannot.
        "page-break-before" => f
            .break_before
            .as_ref()
            .and_then(specified)
            .and_then(|v| page_break_between_keyword(*v))
            .map(str::to_string),
        "page-break-after" => f
            .break_after
            .as_ref()
            .and_then(specified)
            .and_then(|v| page_break_between_keyword(*v))
            .map(str::to_string),
        "page-break-inside" => f
            .break_inside
            .as_ref()
            .and_then(specified)
            .and_then(|v| match v {
                crate::layout::BreakInside::Auto => Some("auto"),
                crate::layout::BreakInside::Avoid => Some("avoid"),
                _ => None,
            })
            .map(str::to_string),
        "orphans" => f.orphans.as_ref().and_then(specified).map(u32::to_string),
        "widows" => f.widows.as_ref().and_then(specified).map(u32::to_string),
        "box-decoration-break" => f
            .box_decoration_break
            .as_ref()
            .and_then(specified)
            .map(|v| v.keyword().to_string()),
        _ => return None,
    })
}
