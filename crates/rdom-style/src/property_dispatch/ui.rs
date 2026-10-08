//! The CSS UI 4 properties: the outline (§5) — `outline` and its four
//! longhands — `cursor` (§4.1), and the caret's `caret-shape`,
//! `caret-animation` and the `caret` shorthand (§6.2; `caret-color` is
//! `set.rs`'s): their `set` and `serialize` arms.

use super::border::{serialize_line_width, serialize_paint_length};
use super::value_serializers::{serialize_color, specified};
use crate::layout::{
    AccentColor, BorderWidth, CaretAnimation, CaretColor, CaretShape, OutlineColor, OutlineStyle,
};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_accent_color, parse_appearance, parse_caret, parse_caret_animation, parse_caret_shape,
    parse_cursor, parse_field_sizing, parse_line_width, parse_outline, parse_outline_color,
    parse_outline_offset, parse_outline_style, parse_resize,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    if name == "caret" {
        return Some(parse_caret(value).map(|(color, animation, shape)| {
            style.ui.caret_animation = Some(Value::Specified(animation));
            style.ui.caret_shape = Some(Value::Specified(shape));
            style.caret_color = Some(Value::Specified(color));
        }));
    }
    let ui = &mut style.ui;
    Some(match name {
        "outline" => parse_outline(value).map(|(color, s, width)| {
            ui.outline_color = Some(Value::Specified(color));
            ui.outline_style = Some(Value::Specified(s));
            ui.outline_width = Some(Value::Specified(width));
        }),
        "outline-style" => parse_outline_style(value).map(|s| {
            ui.outline_style = Some(Value::Specified(s));
        }),
        "outline-width" => parse_line_width(value).map(|w| {
            ui.outline_width = Some(Value::Specified(w));
        }),
        "outline-color" => parse_outline_color(value).map(|c| {
            ui.outline_color = Some(Value::Specified(c));
        }),
        "outline-offset" => parse_outline_offset(value).map(|o| {
            ui.outline_offset = Some(Value::Specified(o));
        }),
        "cursor" => parse_cursor(value).map(|c| {
            ui.cursor = Some(Value::Specified(c));
        }),
        "caret-shape" => parse_caret_shape(value).map(|s| {
            ui.caret_shape = Some(Value::Specified(s));
        }),
        "caret-animation" => parse_caret_animation(value).map(|a| {
            ui.caret_animation = Some(Value::Specified(a));
        }),
        "accent-color" => parse_accent_color(value).map(|a| {
            ui.accent_color = Some(Value::Specified(a));
        }),
        // §7.1; `-webkit-appearance` is its legacy name (Compat §5).
        "appearance" | "-webkit-appearance" => parse_appearance(value).map(|a| {
            ui.appearance = Some(Value::Specified(a));
        }),
        "field-sizing" => parse_field_sizing(value).map(|f| {
            ui.field_sizing = Some(Value::Specified(f));
        }),
        "resize" => parse_resize(value).map(|r| {
            ui.resize = Some(Value::Specified(r));
        }),
        _ => return None,
    })
}

fn color_text(c: &OutlineColor) -> String {
    match c {
        OutlineColor::Auto => "auto".to_string(),
        OutlineColor::Color(c) => serialize_color(c),
    }
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let ui = &style.ui;
    let s = ui.outline_style.as_ref().and_then(specified);
    let w = ui.outline_width.as_ref().and_then(specified);
    let c = ui.outline_color.as_ref().and_then(specified);
    Some(match name {
        "outline-style" => s.map(|s| s.keyword().to_string()),
        "outline-width" => w.map(serialize_line_width),
        "outline-color" => c.map(color_text),
        "outline-offset" => ui
            .outline_offset
            .as_ref()
            .and_then(specified)
            .map(serialize_paint_length),
        "cursor" => ui.cursor.as_ref().and_then(specified).map(serialize_cursor),
        "caret-shape" => ui
            .caret_shape
            .as_ref()
            .and_then(specified)
            .map(|s| s.keyword().to_string()),
        "caret-animation" => ui
            .caret_animation
            .as_ref()
            .and_then(specified)
            .map(|a| a.keyword().to_string()),
        "resize" => ui
            .resize
            .as_ref()
            .and_then(specified)
            .map(|r| r.keyword().to_string()),
        "field-sizing" => ui
            .field_sizing
            .as_ref()
            .and_then(specified)
            .map(|f| f.keyword().to_string()),
        "appearance" | "-webkit-appearance" => ui
            .appearance
            .as_ref()
            .and_then(specified)
            .map(|a| a.keyword().to_string()),
        "accent-color" => ui
            .accent_color
            .as_ref()
            .and_then(specified)
            .map(|a| match a {
                AccentColor::Auto => "auto".to_string(),
                AccentColor::Color(c) => serialize_color(c),
            }),
        // The shortest form: the components off their initial `auto`, in
        // grammar order; `auto` when all are.
        "caret" => {
            let color = style.caret_color.as_ref().and_then(specified);
            let animation = ui.caret_animation.as_ref().and_then(specified);
            let shape = ui.caret_shape.as_ref().and_then(specified);
            match (color, animation, shape) {
                (Some(c), Some(a), Some(s)) => {
                    let mut parts = Vec::new();
                    if *c != CaretColor::Auto {
                        parts.push(match c {
                            CaretColor::Color(c) => serialize_color(c),
                            _ => "transparent".to_string(),
                        });
                    }
                    if *a != CaretAnimation::Auto {
                        parts.push(a.keyword().to_string());
                    }
                    if *s != CaretShape::Auto {
                        parts.push(s.keyword().to_string());
                    }
                    Some(if parts.is_empty() {
                        "auto".to_string()
                    } else {
                        parts.join(" ")
                    })
                }
                _ => None,
            }
        }
        // The shortest form (CSSOM §6.7.2): the components that are not
        // at their initial value, color, style, width; `none` when all
        // are.
        "outline" => match (c, s, w) {
            (Some(c), Some(s), Some(w)) => {
                let mut parts = Vec::new();
                if *c != OutlineColor::Auto {
                    parts.push(color_text(c));
                }
                if *s != OutlineStyle::None {
                    parts.push(s.keyword().to_string());
                }
                if *w != BorderWidth::Medium {
                    parts.push(serialize_line_width(w));
                }
                Some(if parts.is_empty() {
                    "none".to_string()
                } else {
                    parts.join(" ")
                })
            }
            _ => None,
        },
        _ => return None,
    })
}

/// `cursor` as CSS text: each image (a URL string, its hotspot), then the
/// keyword.
fn serialize_cursor(c: &crate::layout::Cursor) -> String {
    let mut parts: Vec<String> = c
        .images
        .iter()
        .map(|i| {
            let url = format!("url({})", rdom_core::css_syntax::serialize_string(&i.url));
            match i.hotspot {
                Some((x, y)) => format!("{url} {x} {y}"),
                None => url,
            }
        })
        .collect();
    parts.push(c.keyword.keyword().to_string());
    parts.join(", ")
}
