//! Declaring a property on a block: [`set`], [`set_from_source`],
//! [`set_from_tokens`] and the custom-property paths. A value holding
//! `var()` / `attr()`, or an inline-axis flow-relative property, is kept
//! as written on the block's `pending` list for the cascade (with the
//! declarations after it, in order); everything else is parsed now by
//! `set::set_parsed`.

use super::DispatchError;
use super::css_wide::css_wide_keyword;
use super::set::set_parsed;
use super::table::canonical_property_name;
use crate::TuiStyle;
use crate::layout::TextDirection;
use crate::parse::token::{Token, tokenize};

/// Set `name = value` on `style`. Tokenizes the value first; for
/// callers that already have tokens, prefer [`set_from_source`] (or
/// [`set_from_tokens`]). The text is kept, trimmed, for a value CSSOM
/// reads back as written.
pub fn set(name: &str, value: &str, style: &mut TuiStyle) -> Result<(), DispatchError> {
    let tokens = tokenize(value).map_err(|_| DispatchError::InvalidValue)?;
    set_with_text(name, &tokens, Some(value.trim()), false, style)
}

/// [`set_from_tokens`] with the value's `text` as written (its source,
/// trimmed), when the caller has it, and the declaration's
/// `!important` — the shape of [`set_custom_source`]: a front end
/// declares one parsed declaration in one call. A custom property keeps
/// the text as its value (CSS Variables 1 §2), and a declaration holding
/// `var()` / `attr()` keeps it until the cascade substitutes it (§3), so
/// CSSOM reads both back as written rather than re-serialized from
/// tokens (`--x: 1 - 2`, not `1 -2`). `important` marks the declaration
/// `!important` (CSS Cascade 4 §6.4): the property's fields, a kept
/// declaration's own flag, or the custom property's.
pub fn set_from_source(
    name: &str,
    value: &[Token],
    text: Option<&str>,
    important: bool,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    set_with_text(name, value, text, important, style)
}

/// Pre-tokenized variant of [`set`]. The block parser in
/// `rdom-css` calls this to avoid re-tokenizing each declaration's
/// value when the surrounding block was already tokenized.
///
/// A value containing `var()` (CSS Variables 1 §3) is checked for
/// `var()` syntax only and kept as tokens on `style.pending`, for the
/// cascade to substitute and parse per element; once a block holds one,
/// later declarations are recorded there too so the cascade replays
/// them in order.
pub fn set_from_tokens(
    name: &str,
    value: &[Token],
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    set_with_text(name, value, None, false, style)
}

/// The one body of [`set`], [`set_from_tokens`] and [`set_from_source`]:
/// `text` is the value as written, when the caller has it, and
/// `important` the declaration's priority.
fn set_with_text(
    name: &str,
    value: &[Token],
    text: Option<&str>,
    important: bool,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    if let Some(custom) = name.strip_prefix("--") {
        if custom.is_empty() {
            return Err(DispatchError::UnknownProperty);
        }
        return set_custom_source(custom, value, text, important, style);
    }
    declare(name, value, text, style)?;
    if important {
        super::set_important(name, true, style);
    }
    Ok(())
}

/// Declare the non-custom property `name` (normal priority).
fn declare(
    name: &str,
    value: &[Token],
    text: Option<&str>,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    let name = &*canonical_property_name(name);
    if crate::var::contains_substitution(value) {
        if super::table::fields_of(name).is_none() {
            return Err(DispatchError::UnknownProperty);
        }
        if !crate::var::valid_var_syntax(value) {
            return Err(DispatchError::InvalidValue);
        }
        style.pending.retain(|d| d.name != name);
        let mut kept = crate::var::PendingDeclaration::new(name, value, true);
        kept.text = text.map(Box::from);
        style.pending.push(kept);
        return Ok(());
    }
    // CSS Logical 1 §4: an inline-axis property maps by the element's
    // `direction`, which only the cascade knows — kept as written (its
    // value checked now), with the block's later declarations, for the
    // cascade to replay in order (`logical.rs`).
    if super::logical::is_directional(name) {
        super::logical::set_mapped(name, value, &mut TuiStyle::new(), TextDirection::Ltr)
            .unwrap_or(Err(DispatchError::UnknownProperty))?;
        // A CSS-wide keyword is kept in its canonical spelling.
        let value = match value {
            [Token::Ident(kw)] if css_wide_keyword(value).is_some() => {
                vec![Token::Ident(kw.to_ascii_lowercase())]
            }
            _ => value.to_vec(),
        };
        style.pending.retain(|d| d.name != name);
        style
            .pending
            .push(crate::var::PendingDeclaration::new(name, &value, false));
        return Ok(());
    }
    set_parsed(name, value, style)?;
    if style.has_pending() {
        style.pending.retain(|d| d.name != name);
        if style
            .pending
            .iter()
            .any(|d| d.has_substitution || d.directional)
        {
            style
                .pending
                .push(crate::var::PendingDeclaration::new(name, value, false));
        } else {
            style.pending.clear();
        }
    }
    Ok(())
}

/// Declare the custom property `--name` (`name` without the dashes) as
/// `value`, `!important` when `important` — the one path for a parsed
/// custom property, the block parser's and `set`'s. The value is any
/// token sequence (CSS Variables 1 §2) except one holding a
/// `<bad-url-token>` (§2.1), which is `InvalidValue`.
pub fn set_custom(
    name: &str,
    value: &[Token],
    important: bool,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    set_custom_source(name, value, None, important, style)
}

/// [`set_custom`] keeping `text`, the value as written (trimmed), when
/// the caller has it: that is the value (CSS Variables 1 §2), which
/// CSSOM reads back. Without it the text is the tokens' serialization.
pub fn set_custom_source(
    name: &str,
    value: &[Token],
    text: Option<&str>,
    important: bool,
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    if value.contains(&Token::BadUrl) {
        return Err(DispatchError::InvalidValue);
    }
    let rendered;
    let text = match text {
        Some(t) => t,
        None => {
            rendered = crate::parse::values::render_value(value);
            &rendered
        }
    };
    style.set_custom_property(name, text, important);
    Ok(())
}
