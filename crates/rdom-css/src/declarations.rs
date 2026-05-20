//! Declaration-block parser. Consumes the body string between `{`
//! and `}` and writes onto a `TuiStyle`.
//!
//! Per-property setter logic lives in
//! [`rdom_style::property_dispatch`]; this module owns the
//! block-shape parsing only (tokenize → split on `;` → strip
//! `!important` → delegate one declaration at a time).

use rdom_style::TuiStyle;
use rdom_style::parse::token::{Token, TokenizerErrorKind, tokenize};
use rdom_style::parse::values::render_value;
use rdom_style::property_dispatch::{self, DispatchError};

use crate::{Warning, WarningKind};

/// Parse a declaration block. `block_line` / `block_col` mark the
/// start of the body in the original source — they're used as a
/// fallback location when token-level position info isn't tracked
/// here yet.
///
/// Custom-property declarations (`--name: value`) are routed into
/// `custom_props` instead of `style`. The caller decides what to
/// do with them — currently only `:root` rules feed them to
/// `Stylesheet::define_var`.
pub(crate) fn parse_block(
    body: &str,
    style: &mut TuiStyle,
    custom_props: &mut Vec<CustomProperty>,
    block_line: u32,
    block_col: u32,
    warnings: &mut Vec<Warning>,
) {
    let tokens = match tokenize(body) {
        Ok(t) => t,
        Err(e) => {
            let kind = match e.kind {
                TokenizerErrorKind::UnterminatedComment => WarningKind::UnterminatedComment,
                TokenizerErrorKind::UnterminatedString => WarningKind::UnterminatedString,
            };
            warnings.push(Warning {
                kind,
                line: e.line,
                column: e.column,
            });
            return;
        }
    };
    let mut decls = split_declarations(&tokens);
    for decl in decls.drain(..) {
        if let Some(name) = decl.name.strip_prefix("--") {
            // Custom property — skip the property table, route
            // out for the caller to register.
            custom_props.push(CustomProperty {
                name: name.to_string(),
                value: render_value(decl.value),
            });
            continue;
        }
        apply_declaration(decl, style, block_line, block_col, warnings);
    }
}

/// A `--name: value` declaration captured during block parsing.
/// `value` is the verbatim source text (rendered from tokens).
#[derive(Debug)]
pub(crate) struct CustomProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
struct RawDeclaration<'a> {
    name: &'a str,
    value: &'a [Token],
    important: bool,
}

/// Split a token slice on top-level `;`s. Each non-empty segment
/// must contain `name : value …` — a leading ident followed by `:`.
/// Segments that don't match emit a warning at apply time.
fn split_declarations(tokens: &[Token]) -> Vec<RawDeclaration<'_>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let len = tokens.len();
    let mut i = 0usize;
    while i <= len {
        let at_end = i == len;
        if at_end || tokens[i] == Token::Semicolon {
            let segment = &tokens[start..i];
            if let Some(decl) = into_declaration(segment) {
                out.push(decl);
            }
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    out
}

fn into_declaration(segment: &[Token]) -> Option<RawDeclaration<'_>> {
    if segment.is_empty() {
        return None;
    }
    let name = match &segment[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    if segment.len() < 2 || segment[1] != Token::Colon {
        return None;
    }
    let mut value: &[Token] = &segment[2..];
    let important = strip_trailing_important(&mut value);
    Some(RawDeclaration {
        name,
        value,
        important,
    })
}

/// Detect `… !important` at the end of a value-token slice; if
/// found, mutate `value` to point past those two tokens and
/// return `true`. Whitespace between `!` and `important` is
/// already eaten by the tokenizer; case-insensitive on the
/// keyword.
fn strip_trailing_important(value: &mut &[Token]) -> bool {
    if value.len() < 2 {
        return false;
    }
    let last_idx = value.len() - 1;
    let bang_idx = value.len() - 2;
    let is_important = match &value[last_idx] {
        Token::Ident(s) => s.eq_ignore_ascii_case("important"),
        _ => false,
    };
    if value[bang_idx] == Token::Bang && is_important {
        *value = &value[..bang_idx];
        return true;
    }
    false
}

fn apply_declaration(
    decl: RawDeclaration,
    style: &mut TuiStyle,
    line: u32,
    column: u32,
    warnings: &mut Vec<Warning>,
) {
    let name = decl.name;
    let value = decl.value;

    // Single source of truth: rdom_style::property_dispatch owns
    // the name→setter table. The block parser is now a thin
    // tokenizer + per-declaration loop on top of that.
    match property_dispatch::set_from_tokens(name, value, style) {
        Ok(()) => {
            if decl.important
                && let Some(mask) = property_dispatch::property_mask(name)
            {
                style.important |= mask;
            }
        }
        Err(DispatchError::UnknownProperty) => {
            warnings.push(Warning {
                kind: WarningKind::UnknownProperty(name.to_string()),
                line,
                column,
            });
        }
        Err(DispatchError::InvalidValue) => {
            let value_text = render_value(value);
            warnings.push(Warning {
                kind: WarningKind::InvalidValue {
                    property: name.to_string(),
                    value: value_text,
                },
                line,
                column,
            });
        }
    }
}
