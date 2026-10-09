//! Containment: `contain-intrinsic-size` and its longhands (CSS Sizing 4
//! §6.1) — the logical longhands write the physical fields
//! (horizontal-tb: inline is width, block is height) — and the
//! query-container properties `container-type`, `container-name` and
//! `container` (CSS Conditional 5 §6.1–§6.3): their `set` and `serialize`
//! arms.

use super::value_serializers::{serialize_math, specified};
use crate::calc::CalcExpr;
use crate::layout::{ContainIntrinsicSize, ContainerName};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_contain, parse_contain_intrinsic, parse_container, parse_container_name,
    parse_container_type, parse_will_change,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the five names. `None` when `name` is not
/// one; `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let spec = |v: ContainIntrinsicSize| Some(Value::Specified(v));
    Some(match name {
        "contain-intrinsic-size" => parse_contain_intrinsic(value, 2).map(|v| {
            let mut v = v.into_iter();
            let width = v.next().unwrap_or_default();
            let height = v.next().unwrap_or_else(|| width.clone());
            style.contain_intrinsic_width = spec(width);
            style.contain_intrinsic_height = spec(height);
        }),
        "contain-intrinsic-width" | "contain-intrinsic-inline-size" => {
            parse_contain_intrinsic(value, 1).map(|mut v| {
                style.contain_intrinsic_width = spec(v.remove(0));
            })
        }
        "contain-intrinsic-height" | "contain-intrinsic-block-size" => {
            parse_contain_intrinsic(value, 1).map(|mut v| {
                style.contain_intrinsic_height = spec(v.remove(0));
            })
        }
        "contain" => parse_contain(value).map(|c| {
            style.contain = Some(Value::Specified(c));
        }),
        "content-visibility" => crate::parse::values::parse_keyword(
            value,
            &[
                ("visible", crate::layout::ContentVisibility::Visible),
                ("auto", crate::layout::ContentVisibility::Auto),
                ("hidden", crate::layout::ContentVisibility::Hidden),
            ],
        )
        .map(|v| {
            style.content_visibility = Some(Value::Specified(v));
        }),
        "will-change" => parse_will_change(value).map(|w| {
            style.will_change = Some(Value::Specified(w));
        }),
        "container-type" => parse_container_type(value).map(|t| {
            style.container_type = Some(Value::Specified(t));
        }),
        "container-name" => parse_container_name(value).map(|n| {
            style.container_name = Some(Value::Specified(n));
        }),
        "container" => parse_container(value).map(|(n, t)| {
            style.container_name = Some(Value::Specified(n));
            style.container_type = Some(Value::Specified(t));
        }),
        _ => return None,
    })
}

/// Serialize one of the five names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "contain-intrinsic-size" => {
            match (
                field(&style.contain_intrinsic_width),
                field(&style.contain_intrinsic_height),
            ) {
                (Some(w), Some(h)) if w == h => Some(serialize_one(w)),
                (Some(w), Some(h)) => Some(format!("{} {}", serialize_one(w), serialize_one(h))),
                _ => None,
            }
        }
        "contain-intrinsic-width" | "contain-intrinsic-inline-size" => {
            field(&style.contain_intrinsic_width).map(serialize_one)
        }
        "contain-intrinsic-height" | "contain-intrinsic-block-size" => {
            field(&style.contain_intrinsic_height).map(serialize_one)
        }
        "contain" => style.contain.as_ref().and_then(specified).map(|c| c.css()),
        "content-visibility" => style
            .content_visibility
            .as_ref()
            .and_then(specified)
            .map(|v| v.css().to_string()),
        "will-change" => style.will_change.as_ref().and_then(specified).map(|w| {
            if w.features().is_empty() {
                "auto".to_string()
            } else {
                w.features()
                    .iter()
                    .map(|f| f.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }),
        "container-type" => style
            .container_type
            .as_ref()
            .and_then(specified)
            .map(|t| t.css().to_string()),
        "container-name" => style
            .container_name
            .as_ref()
            .and_then(specified)
            .map(name_text),
        "container" => {
            let name = style.container_name.as_ref().and_then(specified);
            let kind = style.container_type.as_ref().and_then(specified);
            match (name, kind) {
                (Some(n), Some(t)) if *t == Default::default() => Some(name_text(n)),
                (Some(n), Some(t)) => Some(format!("{} / {}", name_text(n), t.css())),
                _ => None,
            }
        }
        _ => return None,
    })
}

/// `none` or the names, space-separated.
fn name_text(n: &ContainerName) -> String {
    if n.is_none() {
        "none".to_string()
    } else {
        n.names()
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The specified value of one of the fields.
fn field(f: &Option<Value<ContainIntrinsicSize>>) -> Option<&ContainIntrinsicSize> {
    f.as_ref().and_then(specified)
}

/// `auto? [ none | <length> ]`.
fn serialize_one(v: &ContainIntrinsicSize) -> String {
    let length = match &v.length {
        None => "none".to_string(),
        Some(CalcExpr::Length(n)) => n.to_string(),
        Some(expr) => serialize_math(expr),
    };
    if v.auto {
        format!("auto {length}")
    } else {
        length
    }
}
