//! `clip-path` (CSS Masking 1 §5.1) and the mask properties (§6–§7): their
//! `set` and `serialize` arms.

use super::value_serializers::{serialize_css_string, serialize_length, specified};
use crate::layout::{BasicShape, ClipPath, GeometryBox, Length, ShapeRadius};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_clip_path, parse_mask_border_shorthand, parse_mask_longhand, parse_mask_shorthand,
};
use crate::{TuiStyle, Value};

/// The mask longhand `name`'s field.
fn mask_field<'a>(name: &str, style: &'a mut TuiStyle) -> Option<&'a mut Option<Value<String>>> {
    let m = &mut style.masks;
    Some(match name {
        "mask-image" => &mut m.mask_image,
        "mask-mode" => &mut m.mask_mode,
        "mask-repeat" => &mut m.mask_repeat,
        "mask-position" => &mut m.mask_position,
        "mask-clip" => &mut m.mask_clip,
        "mask-origin" => &mut m.mask_origin,
        "mask-size" => &mut m.mask_size,
        "mask-composite" => &mut m.mask_composite,
        "mask-type" => &mut m.mask_type,
        "mask-border-source" => &mut m.mask_border_source,
        "mask-border-slice" => &mut m.mask_border_slice,
        "mask-border-width" => &mut m.mask_border_width,
        "mask-border-outset" => &mut m.mask_border_outset,
        "mask-border-repeat" => &mut m.mask_border_repeat,
        "mask-border-mode" => &mut m.mask_border_mode,
        _ => return None,
    })
}

/// Parse and write `clip-path`, a mask longhand or a mask shorthand.
/// `None` when `name` is none of them; `Some(None)` when its value is
/// invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    if name == "clip-path" {
        return Some(
            parse_clip_path(value).map(|v| style.effects.clip_path = Some(Value::Specified(v))),
        );
    }
    let shorthand = match name {
        "mask" => Some(parse_mask_shorthand(value)),
        "mask-border" => Some(parse_mask_border_shorthand(value)),
        _ => None,
    };
    if let Some(parsed) = shorthand {
        return Some(parsed.map(|longhands| {
            for (n, text) in longhands {
                if let Some(field) = mask_field(n, style) {
                    *field = Some(Value::Specified(text));
                }
            }
        }));
    }
    let parsed = parse_mask_longhand(name, value);
    let field = mask_field(name, style)?;
    Some(parsed.map(|text| *field = Some(Value::Specified(text))))
}

/// Serialize `clip-path` or a mask property. `None` when `name` is none
/// of them.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    if name == "clip-path" {
        return Some(
            style
                .effects
                .clip_path
                .as_ref()
                .and_then(specified)
                .map(serialize_clip_path),
        );
    }
    let mut copy = style.clone();
    let text = |n: &str, copy: &mut TuiStyle| {
        mask_field(n, copy).and_then(|f| f.as_ref().and_then(specified).cloned())
    };
    match name {
        // A shorthand of one layer, every longhand set: its longhands
        // in the shorthand's order.
        "mask" => {
            let order = [
                "mask-image",
                "mask-position",
                "mask-size",
                "mask-repeat",
                "mask-origin",
                "mask-clip",
                "mask-composite",
                "mask-mode",
            ];
            let v: Option<Vec<String>> = order.iter().map(|n| text(n, &mut copy)).collect();
            Some(v.filter(|v| v.iter().all(|t| !t.contains(','))).map(|v| {
                format!(
                    "{} {} / {} {} {} {} {} {}",
                    v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
                )
            }))
        }
        "mask-border" => {
            let order = [
                "mask-border-source",
                "mask-border-slice",
                "mask-border-width",
                "mask-border-outset",
                "mask-border-repeat",
                "mask-border-mode",
            ];
            let v: Option<Vec<String>> = order.iter().map(|n| text(n, &mut copy)).collect();
            Some(v.map(|v| format!("{} {} / {} / {} {} {}", v[0], v[1], v[2], v[3], v[4], v[5])))
        }
        _ => {
            mask_field(name, &mut copy)?;
            Some(text(name, &mut copy))
        }
    }
}

/// `clip-path` in its shortest form: the reference box only when not
/// `border-box` (or alone).
fn serialize_clip_path(c: &ClipPath) -> String {
    match c {
        ClipPath::None => "none".to_string(),
        ClipPath::Url(u) => format!("url({})", serialize_css_string(u)),
        ClipPath::Shape { shape, reference } => match shape {
            None => reference.keyword().to_string(),
            Some(s) if *reference == GeometryBox::BorderBox => serialize_shape(s),
            Some(s) => format!("{} {}", serialize_shape(s), reference.keyword()),
        },
    }
}

fn radius(r: &ShapeRadius) -> String {
    match r {
        ShapeRadius::Length(l) => serialize_length(l),
        ShapeRadius::ClosestSide => "closest-side".to_string(),
        ShapeRadius::FarthestSide => "farthest-side".to_string(),
    }
}

/// The shortest of the 1–4 sides (or corners) form.
fn sides(v: &[Length; 4]) -> String {
    let [a, b, c, d] = v.each_ref().map(serialize_length);
    if b == d {
        if a == c {
            if a == b { a } else { format!("{a} {b}") }
        } else {
            format!("{a} {b} {c}")
        }
    } else {
        format!("{a} {b} {c} {d}")
    }
}

fn serialize_shape(s: &BasicShape) -> String {
    let at = |at: &[Length; 2]| {
        let centre = [
            Length::calc(crate::calc::CalcExpr::Percent(50.0)),
            Length::calc(crate::calc::CalcExpr::Percent(50.0)),
        ];
        if *at == centre {
            String::new()
        } else {
            format!(
                "at {} {}",
                serialize_length(&at[0]),
                serialize_length(&at[1])
            )
        }
    };
    let join = |parts: Vec<String>| {
        parts
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    match s {
        BasicShape::Inset { insets, round } => {
            let zero = round.iter().all(|r| *r == Length::Cells(0));
            if zero {
                format!("inset({})", sides(insets))
            } else {
                format!("inset({} round {})", sides(insets), sides(round))
            }
        }
        BasicShape::Circle { radius: r, at: p } => {
            let r = if *r == ShapeRadius::ClosestSide {
                String::new()
            } else {
                radius(r)
            };
            format!("circle({})", join(vec![r, at(p)]))
        }
        BasicShape::Ellipse { rx, ry, at: p } => {
            let both = *rx == ShapeRadius::ClosestSide && *ry == ShapeRadius::ClosestSide;
            let r = if both {
                String::new()
            } else {
                format!("{} {}", radius(rx), radius(ry))
            };
            format!("ellipse({})", join(vec![r, at(p)]))
        }
        BasicShape::Polygon { evenodd, points } => {
            let mut parts: Vec<String> = Vec::new();
            if *evenodd {
                parts.push("evenodd".to_string());
            }
            parts.extend(
                points
                    .iter()
                    .map(|[x, y]| format!("{} {}", serialize_length(x), serialize_length(y))),
            );
            format!("polygon({})", parts.join(", "))
        }
        BasicShape::Path { evenodd, data } => {
            if *evenodd {
                format!("path(evenodd, {})", serialize_css_string(data))
            } else {
                format!("path({})", serialize_css_string(data))
            }
        }
    }
}
