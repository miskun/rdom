//! Declaration-block parser. Consumes the body string between `{`
//! and `}` and writes onto a `TuiStyle`.
//!
//! Per-property setter logic lives in
//! [`rdom_style::property_dispatch`]; this module owns the
//! block-shape parsing only (tokenize → split on `;` → strip
//! `!important` → delegate one declaration at a time).

use rdom_style::TuiStyle;
use rdom_style::parse::token::{Token, TokenPos, TokenizerErrorKind, tokenize_at};
use rdom_style::parse::values::render_value;
use rdom_style::property_dispatch::{self, DispatchError};

use crate::{Warning, WarningKind};

/// Parse a declaration block. `block_line` / `block_col` mark the
/// start of the body in the original source; every warning below
/// carries the position of the declaration (or token) it is about.
///
/// Custom-property declarations (`--name: value`) land on
/// `style.custom_properties` like any other declaration; the cascade
/// scopes them per element (CSS Variables 1).
pub(crate) fn parse_block(
    body: &str,
    style: &mut TuiStyle,
    block_line: u32,
    block_col: u32,
    warnings: &mut Vec<Warning>,
) {
    let (tokens, positions) = match tokenize_at(body, block_line, block_col) {
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
    let mut decls = split_declarations(&tokens, &positions, warnings);
    for decl in decls.drain(..) {
        if let Some(name) = decl.name.strip_prefix("--") {
            // Custom property: untyped, kept verbatim, importance per
            // declaration.
            style.set_custom_property(name, &render_value(decl.value), decl.important);
            continue;
        }
        apply_declaration(decl, style, warnings);
    }
}

#[derive(Debug)]
struct RawDeclaration<'a> {
    name: &'a str,
    value: &'a [Token],
    important: bool,
    /// Position of the property name in the source.
    at: TokenPos,
}

/// Split a token slice on top-level `;`s. Each non-empty segment
/// must contain `name : value …` — a leading ident followed by `:`.
/// A non-empty segment that doesn't match is dropped (CSS Syntax 3
/// §5.4.4) with a `MalformedDeclaration` warning; empty segments
/// (`;;`, trailing `;`) are silently fine.
fn split_declarations<'a>(
    tokens: &'a [Token],
    positions: &[TokenPos],
    warnings: &mut Vec<Warning>,
) -> Vec<RawDeclaration<'a>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let len = tokens.len();
    let mut i = 0usize;
    while i <= len {
        let at_end = i == len;
        if at_end || tokens[i] == Token::Semicolon {
            let segment = &tokens[start..i];
            let at = positions.get(start).copied().unwrap_or((0, 0));
            match into_declaration(segment, at) {
                Some(decl) => out.push(decl),
                None if !segment.is_empty() => warnings.push(Warning {
                    kind: WarningKind::MalformedDeclaration(render_value(segment)),
                    line: at.0,
                    column: at.1,
                }),
                None => {}
            }
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    out
}

fn into_declaration(segment: &[Token], at: TokenPos) -> Option<RawDeclaration<'_>> {
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
        at,
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

fn apply_declaration(decl: RawDeclaration, style: &mut TuiStyle, warnings: &mut Vec<Warning>) {
    let name = decl.name;
    let value = decl.value;
    let (line, column) = decl.at;

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
