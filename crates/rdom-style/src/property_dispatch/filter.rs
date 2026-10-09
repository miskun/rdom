//! `filter` and `backdrop-filter` (Filter Effects 1 §5–§6, Filter Effects
//! 2 §3): their `set` and `serialize` arms.

use super::border::serialize_paint_length;
use super::value_serializers::{serialize_color, specified};
use crate::layout::{FilterFunction, FilterList, PaintLength};
use crate::parse::token::Token;
use crate::parse::values::parse_filter;
use crate::{TuiColor, TuiStyle, Value};

/// Parse and write either name. `None` when `name` is neither;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let e = &mut style.effects;
    let slot = match name {
        "filter" => &mut e.filter,
        "backdrop-filter" => &mut e.backdrop_filter,
        _ => return None,
    };
    Some(parse_filter(value).map(|v| *slot = Some(Value::Specified(v))))
}

/// Serialize either name. `None` when `name` is neither.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let e = &style.effects;
    let slot = match name {
        "filter" => &e.filter,
        "backdrop-filter" => &e.backdrop_filter,
        _ => return None,
    };
    Some(slot.as_ref().and_then(specified).map(serialize_filter))
}

/// A number in its shortest form (six decimals at most).
fn number(n: f64) -> String {
    let rounded = (n * 1e6).round() / 1e6;
    format!("{}", if rounded == 0.0 { 0.0 } else { rounded })
}

/// `none`, or the functions space-separated, each amount a number.
fn serialize_filter(list: &FilterList) -> String {
    if list.is_none() {
        return "none".to_string();
    }
    list.functions()
        .iter()
        .map(|f| match f {
            FilterFunction::Blur(l) => format!("blur({})", serialize_paint_length(l)),
            FilterFunction::Brightness(a) => format!("brightness({})", number(*a)),
            FilterFunction::Contrast(a) => format!("contrast({})", number(*a)),
            FilterFunction::Grayscale(a) => format!("grayscale({})", number(*a)),
            FilterFunction::HueRotate(d) => format!("hue-rotate({}deg)", number(*d)),
            FilterFunction::Invert(a) => format!("invert({})", number(*a)),
            FilterFunction::Opacity(a) => format!("opacity({})", number(*a)),
            FilterFunction::Saturate(a) => format!("saturate({})", number(*a)),
            FilterFunction::Sepia(a) => format!("sepia({})", number(*a)),
            FilterFunction::DropShadow(s) => {
                let zero = |l: &PaintLength| {
                    matches!(l, PaintLength::Cells(v) | PaintLength::Px(v) if *v == 0.0)
                };
                let mut parts = vec![
                    serialize_paint_length(&s.offset_x),
                    serialize_paint_length(&s.offset_y),
                ];
                if !zero(&s.blur) {
                    parts.push(serialize_paint_length(&s.blur));
                }
                if s.color != TuiColor::CurrentColor {
                    parts.push(serialize_color(&s.color));
                }
                format!("drop-shadow({})", parts.join(" "))
            }
            FilterFunction::Url(u) => format!("url({})", super::value_serializers::serialize_css_string(u)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
