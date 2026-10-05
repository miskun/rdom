//! `contain-intrinsic-size` and its longhands (CSS Sizing 4 §6.1): their
//! `set` and `serialize` arms. The logical longhands write the physical
//! fields (horizontal-tb: inline is width, block is height).

use super::value_serializers::{serialize_math, specified};
use crate::calc::CalcExpr;
use crate::layout::ContainIntrinsicSize;
use crate::parse::token::Token;
use crate::parse::values::parse_contain_intrinsic;
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
        _ => return None,
    })
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
