//! The compositing properties (Compositing and Blending 1 §3.2, §3.4,
//! §5.2): `mix-blend-mode`, `isolation`, `background-blend-mode` — their
//! `set` and `serialize` arms.

use super::value_serializers::specified;
use crate::parse::token::Token;
use crate::parse::values::{parse_background_blend_mode, parse_isolation, parse_mix_blend_mode};
use crate::{TuiStyle, Value};

/// Parse and write one of the three names. `None` when `name` is not
/// one; `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let e = &mut style.effects;
    Some(match name {
        "mix-blend-mode" => {
            parse_mix_blend_mode(value).map(|v| e.mix_blend_mode = Some(Value::Specified(v)))
        }
        "isolation" => parse_isolation(value).map(|v| e.isolation = Some(Value::Specified(v))),
        "background-blend-mode" => parse_background_blend_mode(value)
            .map(|v| e.background_blend_mode = Some(Value::Specified(v))),
        _ => return None,
    })
}

/// Serialize one of the three names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let e = &style.effects;
    Some(match name {
        "mix-blend-mode" => e
            .mix_blend_mode
            .as_ref()
            .and_then(specified)
            .map(|m| m.keyword().to_string()),
        "isolation" => e
            .isolation
            .as_ref()
            .and_then(specified)
            .map(|i| i.keyword().to_string()),
        "background-blend-mode" => {
            e.background_blend_mode
                .as_ref()
                .and_then(specified)
                .map(|list| {
                    list.iter()
                        .map(|m| m.keyword())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
        }
        _ => return None,
    })
}
