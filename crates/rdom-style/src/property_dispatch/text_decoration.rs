//! The CSS Text Decoration 3 / 4 properties: `text-decoration` and its
//! longhands `text-decoration-line` / `-style` / `-color` /
//! `-thickness`, and `text-underline-offset`, `text-underline-position`,
//! `text-decoration-skip-ink` — their `set` and `serialize` arms.

use super::value_serializers::{serialize_color, specified};
use crate::layout::{TextDecorationStyle, TextDecorationThickness};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_color, parse_text_decoration, parse_text_decoration_line, parse_text_decoration_skip_ink,
    parse_text_decoration_style, parse_text_decoration_thickness, parse_text_underline_offset,
    parse_text_underline_position, serialize_decoration_length,
    serialize_text_decoration_thickness,
};
use crate::{TuiColor, TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let d = &mut style.text_decoration;
    let text = &mut style.text;
    Some(match name {
        "text-decoration" => parse_text_decoration(value).map(|s| {
            d.line = Some(Value::Specified(s.line));
            d.style = Some(Value::Specified(s.style));
            d.color = Some(Value::Specified(s.color));
            d.thickness = Some(Value::Specified(s.thickness));
        }),
        "text-decoration-line" => parse_text_decoration_line(value).map(|l| {
            d.line = Some(Value::Specified(l));
        }),
        "text-decoration-style" => parse_text_decoration_style(value).map(|s| {
            d.style = Some(Value::Specified(s));
        }),
        "text-decoration-color" => parse_color(value).map(|c| {
            d.color = Some(Value::Specified(c));
        }),
        "text-decoration-thickness" => parse_text_decoration_thickness(value).map(|t| {
            d.thickness = Some(Value::Specified(t));
        }),
        "text-underline-offset" => parse_text_underline_offset(value).map(|o| {
            text.text_underline_offset = Some(Value::Specified(o));
        }),
        "text-underline-position" => parse_text_underline_position(value).map(|p| {
            text.text_underline_position = Some(Value::Specified(p));
        }),
        "text-decoration-skip-ink" => parse_text_decoration_skip_ink(value).map(|s| {
            text.text_decoration_skip_ink = Some(Value::Specified(s));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let d = &style.text_decoration;
    let text = &style.text;
    Some(match name {
        "text-decoration-line" => d.line.as_ref().and_then(specified).map(|l| l.keywords()),
        "text-decoration-style" => d
            .style
            .as_ref()
            .and_then(specified)
            .map(|s| s.keyword().to_string()),
        "text-decoration-color" => d.color.as_ref().and_then(specified).map(serialize_color),
        "text-decoration-thickness" => d
            .thickness
            .as_ref()
            .and_then(specified)
            .map(serialize_text_decoration_thickness),
        "text-underline-offset" => {
            text.text_underline_offset
                .as_ref()
                .and_then(specified)
                .map(|o| match o {
                    crate::layout::TextUnderlineOffset::Auto => "auto".to_string(),
                    crate::layout::TextUnderlineOffset::Length(l) => serialize_decoration_length(l),
                })
        }
        "text-underline-position" => text
            .text_underline_position
            .as_ref()
            .and_then(specified)
            .map(|p| p.keywords()),
        "text-decoration-skip-ink" => text
            .text_decoration_skip_ink
            .as_ref()
            .and_then(specified)
            .map(|s| s.keyword().to_string()),
        "text-decoration" => serialize_shorthand(style),
        _ => return None,
    })
}

/// The shorthand's shortest serialization (CSSOM §6.7.2): its line
/// keywords, then each other longhand that is not its initial value;
/// `none` when every one is. Only when all four longhands are set.
fn serialize_shorthand(style: &TuiStyle) -> Option<String> {
    let d = &style.text_decoration;
    let line = d.line.as_ref().and_then(specified)?;
    let style_ = d.style.as_ref().and_then(specified)?;
    let color = d.color.as_ref().and_then(specified)?;
    let thickness = d.thickness.as_ref().and_then(specified)?;
    let mut words: Vec<String> = Vec::new();
    if !line.is_none() {
        words.push(line.keywords());
    }
    if *thickness != TextDecorationThickness::Auto {
        words.push(serialize_text_decoration_thickness(thickness));
    }
    if *style_ != TextDecorationStyle::Solid {
        words.push(style_.keyword().to_string());
    }
    if *color != TuiColor::CurrentColor {
        words.push(serialize_color(color));
    }
    Some(if words.is_empty() {
        "none".to_string()
    } else {
        words.join(" ")
    })
}
