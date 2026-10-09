//! The transform properties (CSS Transforms 1 §5–§7, Transforms 2 §6):
//! `translate`, `rotate`, `scale`, `transform`, `transform-origin` and
//! `transform-box` — their `set` and `serialize` arms.

use super::border::serialize_paint_length;
use super::value_serializers::{serialize_length, specified};
use crate::layout::{
    Length, PaintLength, Rotate, Scale, TransformFunction, TransformList, Translate,
    TranslateFunction,
};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_rotate, parse_scale, parse_transform, parse_transform_box, parse_transform_origin,
    parse_translate,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the six names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let e = &mut style.effects;
    Some(match name {
        "translate" => parse_translate(value).map(|v| e.translate = Some(Value::Specified(v))),
        "rotate" => parse_rotate(value).map(|v| e.rotate = Some(Value::Specified(v))),
        "scale" => parse_scale(value).map(|v| e.scale = Some(Value::Specified(v))),
        "transform" => parse_transform(value).map(|v| e.transform = Some(Value::Specified(v))),
        "transform-origin" => {
            parse_transform_origin(value).map(|v| e.transform_origin = Some(Value::Specified(v)))
        }
        "transform-box" => {
            parse_transform_box(value).map(|v| e.transform_box = Some(Value::Specified(v)))
        }
        _ => return None,
    })
}

/// Serialize one of the six names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let e = &style.effects;
    let none = |s: Option<String>| s.unwrap_or_else(|| "none".to_string());
    Some(match name {
        "translate" => e
            .translate
            .as_ref()
            .and_then(specified)
            .map(|t| none(t.as_ref().map(serialize_translate))),
        "rotate" => e
            .rotate
            .as_ref()
            .and_then(specified)
            .map(|r| none(r.as_ref().map(serialize_rotate))),
        "scale" => e
            .scale
            .as_ref()
            .and_then(specified)
            .map(|s| none(s.as_ref().map(serialize_scale))),
        "transform" => e
            .transform
            .as_ref()
            .and_then(specified)
            .map(serialize_transform),
        "transform-origin" => e
            .transform_origin
            .as_ref()
            .and_then(specified)
            .map(serialize_origin),
        "transform-box" => e
            .transform_box
            .as_ref()
            .and_then(specified)
            .map(|b| b.keyword().to_string()),
        _ => return None,
    })
}

fn zero_length(l: &Length) -> bool {
    *l == Length::Cells(0)
}

fn zero_depth(l: &PaintLength) -> bool {
    matches!(l, PaintLength::Cells(v) | PaintLength::Px(v) if *v == 0.0)
}

/// `translate`: the offsets with the trailing zero ones dropped.
fn serialize_translate(t: &Translate) -> String {
    let (x, y) = (serialize_length(&t.x), serialize_length(&t.y));
    if !zero_depth(&t.z) {
        format!("{x} {y} {}", serialize_paint_length(&t.z))
    } else if !zero_length(&t.y) {
        format!("{x} {y}")
    } else {
        x
    }
}

/// `transform-origin`: x and y as lengths or percentages, z when it is
/// not 0.
fn serialize_origin(o: &crate::layout::TransformOrigin) -> String {
    let (x, y) = (
        serialize_paint_length(&o.x()),
        serialize_paint_length(&o.y()),
    );
    let z = o.z();
    if zero_depth(&z) {
        format!("{x} {y}")
    } else {
        format!("{x} {y} {}", serialize_paint_length(&z))
    }
}

/// A number in its shortest form (six decimals at most).
fn number(n: f64) -> String {
    let rounded = (n * 1e6).round() / 1e6;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded}")
}

/// `rotate`: the axis as `x` / `y` / `z` when it is one, then the angle in
/// degrees.
fn serialize_rotate(r: &Rotate) -> String {
    let angle = format!("{}deg", number(r.degrees));
    match r.axis {
        None => angle,
        Some([1.0, 0.0, 0.0]) => format!("x {angle}"),
        Some([0.0, 1.0, 0.0]) => format!("y {angle}"),
        Some([0.0, 0.0, 1.0]) => format!("z {angle}"),
        Some([a, b, c]) => format!("{} {} {} {angle}", number(a), number(b), number(c)),
    }
}

/// `scale`: x, y when it differs from x, z when it is not 1.
fn serialize_scale(s: &Scale) -> String {
    if s.z != 1.0 {
        format!("{} {} {}", number(s.x), number(s.y), number(s.z))
    } else if s.y != s.x {
        format!("{} {}", number(s.x), number(s.y))
    } else {
        number(s.x)
    }
}

/// `transform`: `none`, or the functions space-separated.
pub(crate) fn serialize_transform(list: &TransformList) -> String {
    if list.is_none() {
        return "none".to_string();
    }
    list.functions()
        .iter()
        .map(serialize_function)
        .collect::<Vec<_>>()
        .join(" ")
}

/// One transform function: a translation as written, any other one's kept
/// text.
fn serialize_function(f: &TransformFunction) -> String {
    match f {
        TransformFunction::Translate { function, offset } => {
            let (x, y) = (serialize_length(&offset.x), serialize_length(&offset.y));
            let z = serialize_paint_length(&offset.z);
            let name = function.name();
            match function {
                TranslateFunction::Translate if zero_length(&offset.y) => format!("{name}({x})"),
                TranslateFunction::Translate => format!("{name}({x}, {y})"),
                TranslateFunction::TranslateX => format!("{name}({x})"),
                TranslateFunction::TranslateY => format!("{name}({y})"),
                TranslateFunction::Translate3d => format!("{name}({x}, {y}, {z})"),
                TranslateFunction::TranslateZ => format!("{name}({z})"),
            }
        }
        TransformFunction::Inert { css, .. } => css.to_string(),
    }
}
