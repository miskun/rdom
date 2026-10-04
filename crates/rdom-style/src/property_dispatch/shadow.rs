//! `box-shadow` (CSS Backgrounds 3 §6.1): its `set` and `serialize`
//! arms.

use super::border::serialize_paint_length;
use super::value_serializers::{join_csv, serialize_color, specified};
use crate::layout::{BoxShadow, PaintLength};
use crate::parse::token::Token;
use crate::parse::values::parse_box_shadow;
use crate::{TuiColor, TuiStyle, Value};

/// Parse and write `box-shadow`. `None` when `name` is not it;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    (name == "box-shadow").then(|| {
        parse_box_shadow(value).map(|list| {
            style.box_shadow = Some(Value::Specified(list));
        })
    })
}

/// Serialize `box-shadow`. `None` when `name` is not it.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    (name == "box-shadow").then(|| {
        style
            .box_shadow
            .as_ref()
            .and_then(specified)
            .map(|list| match list.as_slice() {
                [] => "none".to_string(),
                shadows => join_csv(shadows.iter(), serialize_shadow),
            })
    })
}

/// One `<shadow>` in its shortest form: `inset`, the offsets, the blur
/// and spread when not 0 (the blur kept when a spread follows it), the
/// color when not `currentcolor`.
fn serialize_shadow(s: &BoxShadow) -> String {
    let zero =
        |l: &PaintLength| matches!(l, PaintLength::Cells(v) | PaintLength::Px(v) if *v == 0.0);
    let mut parts = Vec::with_capacity(6);
    if s.inset {
        parts.push("inset".to_string());
    }
    parts.push(serialize_paint_length(&s.offset_x));
    parts.push(serialize_paint_length(&s.offset_y));
    let spread = !zero(&s.spread);
    if spread || !zero(&s.blur) {
        parts.push(serialize_paint_length(&s.blur));
    }
    if spread {
        parts.push(serialize_paint_length(&s.spread));
    }
    if s.color != TuiColor::CurrentColor {
        parts.push(serialize_color(&s.color));
    }
    parts.join(" ")
}
