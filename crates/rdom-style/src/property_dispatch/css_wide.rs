//! CSS-wide keywords (CSS Cascade 4 §7): `inherit`, `initial` and
//! `unset` are valid for every property. This file owns their
//! detection in a token stream, writing them across every field a
//! property owns, and the all-fields-agree rule that decides when a
//! shorthand serializes as the keyword.

use super::DispatchError;
use super::table::{fields_of, inherits};
use crate::TuiStyle;
use crate::Value;
use crate::parse::token::Token;

/// The CSS-wide keyword a field holds, if any.
pub(super) fn keyword_of<T>(v: &Option<Value<T>>) -> Option<&'static str> {
    match v {
        Some(Value::Inherit) => Some("inherit"),
        Some(Value::Initial) => Some("initial"),
        _ => None,
    }
}

/// The CSS-wide keywords (CSS Cascade 4 §7). Valid for every property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CssWide {
    Inherit,
    Initial,
    Unset,
}

pub(super) fn css_wide_keyword(value: &[Token]) -> Option<CssWide> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("inherit") => Some(CssWide::Inherit),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("initial") => Some(CssWide::Initial),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("unset") => Some(CssWide::Unset),
        _ => None,
    }
}

impl CssWide {
    /// Resolve `unset` for `name`, then build the field value.
    pub(super) fn into_value<T>(self, name: &str) -> Value<T> {
        match self {
            CssWide::Inherit => Value::Inherit,
            CssWide::Initial => Value::Initial,
            CssWide::Unset => {
                if inherits(name) {
                    Value::Inherit
                } else {
                    Value::Initial
                }
            }
        }
    }
}

/// Set every field `name` owns to the CSS-wide keyword.
pub(super) fn set_css_wide(
    name: &str,
    kw: CssWide,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    let fields = fields_of(name).ok_or(DispatchError::UnknownProperty)?;
    for f in fields {
        f.put_css_wide(style, kw, name);
    }
    Ok(())
}

/// If every field `name` owns holds the *same* CSS-wide keyword, its
/// spelling; otherwise `None` and the per-field serializer decides.
/// Shorthands over mixed fields (`overflow-x: inherit; overflow-y:
/// hidden`) therefore never claim `inherit` for the whole shorthand.
pub(super) fn css_wide_of(name: &str, style: &TuiStyle) -> Option<&'static str> {
    let fields = fields_of(name)?;
    let first = fields.first()?.css_wide(style)?;
    fields
        .iter()
        .all(|f| f.css_wide(style) == Some(first))
        .then_some(first)
}
