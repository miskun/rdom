//! The CSS Text properties (CSS Text 3 / 4): `white-space` and its
//! longhands `white-space-collapse` / `text-wrap-mode` — their `set` and
//! `serialize` arms.

use super::value_serializers::specified;
use crate::layout::{TextWrapMode, WhiteSpace, WhiteSpaceCollapse};
use crate::parse::token::Token;
use crate::parse::values::{parse_text_wrap_mode, parse_white_space, parse_white_space_collapse};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let text = &mut style.text;
    Some(match name {
        "white-space" => parse_white_space(value).map(|(c, m)| {
            text.white_space_collapse = Some(Value::Specified(c));
            text.text_wrap_mode = Some(Value::Specified(m));
        }),
        "white-space-collapse" => parse_white_space_collapse(value).map(|c| {
            text.white_space_collapse = Some(Value::Specified(c));
        }),
        "text-wrap-mode" => parse_text_wrap_mode(value).map(|m| {
            text.text_wrap_mode = Some(Value::Specified(m));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let text = &style.text;
    let collapse = text.white_space_collapse.as_ref().and_then(specified);
    let mode = text.text_wrap_mode.as_ref().and_then(specified);
    Some(match name {
        "white-space-collapse" => collapse.map(|c| c.keyword().to_string()),
        "text-wrap-mode" => mode.map(|m| m.keyword().to_string()),
        "white-space" => match (collapse, mode) {
            (Some(&c), Some(&m)) => Some(white_space_text(c, m)),
            _ => None,
        },
        _ => return None,
    })
}

/// The shortest `white-space` that sets `collapse` and `mode` (CSSOM
/// §6.7.2's shortest-form rule): the keyword the pair spells, else the
/// longhand values that are not initial.
fn white_space_text(collapse: WhiteSpaceCollapse, mode: TextWrapMode) -> String {
    if let Some(w) = WhiteSpace::from_longhands(collapse, mode) {
        return w.keyword().to_string();
    }
    match mode {
        TextWrapMode::Wrap => collapse.keyword().to_string(),
        TextWrapMode::Nowrap => format!("{} {}", collapse.keyword(), mode.keyword()),
    }
}
