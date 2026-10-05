//! Declaration-block parser. Consumes the body string between `{`
//! and `}` and writes onto a `TuiStyle`.
//!
//! Per-property setter logic lives in
//! [`rdom_style::property_dispatch`]; this module owns the
//! block-shape parsing only (tokenize → split on `;` → strip
//! `!important` → delegate one declaration at a time).

use rdom_style::TuiStyle;
use rdom_style::parse::token::{
    SpannedTokens, Token, TokenPos, TokenizerErrorKind, tokenize_spans,
};
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
    let mut run = DeclarationRun::default();
    run.push(body, block_line, block_col, warnings);
    run.apply(style, warnings);
}

/// The declarations of one block, collected and then applied together:
/// CSS Cascade 4 §6.4 sorts a block's declarations by importance before
/// order of appearance, so an important declaration beats a normal one
/// of the same property whatever their order, and among declarations of
/// equal importance the later wins. The block has one slot per
/// property, so [`apply`](Self::apply) writes the normal declarations
/// first, in order, then the important ones, in order — a later normal
/// declaration cannot overwrite an important one, and a shorthand and
/// its longhands resolve per field (`padding: 1 !important;
/// padding-left: 5` keeps 1 on every side).
#[derive(Default)]
pub(crate) struct DeclarationRun {
    decls: Vec<OwnedDeclaration>,
    /// `warnings.len()` at the first push: the warnings from there on
    /// are re-sorted into source order by `apply`.
    first_warning: Option<usize>,
}

impl DeclarationRun {
    /// Tokenize `body` (a declaration list starting at `line:col`) and
    /// collect its declarations; a malformed one warns now.
    pub(crate) fn push(&mut self, body: &str, line: u32, col: u32, warnings: &mut Vec<Warning>) {
        self.first_warning.get_or_insert(warnings.len());
        let SpannedTokens {
            tokens,
            positions,
            spans,
        } = match tokenize_spans(body, line, col) {
            Ok(t) => t,
            Err(e) => {
                let kind = match e.kind {
                    TokenizerErrorKind::UnterminatedComment => WarningKind::UnterminatedComment,
                    TokenizerErrorKind::UnterminatedString => WarningKind::UnterminatedString,
                    // `TokenizerErrorKind` is `#[non_exhaustive]`; a kind
                    // added upstream without a warning of its own drops the
                    // block as malformed (and trips this assert in the
                    // workspace's tests until it gets one).
                    other => {
                        debug_assert!(false, "unmapped tokenizer error {other:?}");
                        WarningKind::MalformedDeclaration(body.to_string())
                    }
                };
                warnings.push(Warning {
                    kind,
                    line: e.line,
                    column: e.column,
                });
                return;
            }
        };
        let source = Source {
            body,
            positions: &positions,
            spans: &spans,
        };
        let decls = split_declarations(&tokens, &source, warnings);
        self.decls
            .extend(decls.into_iter().map(|d| OwnedDeclaration {
                name: d.name.to_string(),
                value: d.value.to_vec(),
                text: d.text.to_string(),
                important: d.important,
                at: d.at,
            }));
    }

    /// Write the collected declarations onto `style`: normal ones, then
    /// important ones (type doc). Warnings keep source order.
    pub(crate) fn apply(self, style: &mut TuiStyle, warnings: &mut Vec<Warning>) {
        let (important, normal): (Vec<_>, Vec<_>) =
            self.decls.into_iter().partition(|d| d.important);
        for decl in normal.iter().chain(&important) {
            let decl = RawDeclaration {
                name: &decl.name,
                value: &decl.value,
                text: &decl.text,
                important: decl.important,
                at: decl.at,
            };
            if let Some(name) = decl.name.strip_prefix("--") {
                // Custom property: untyped, kept as written (CSS
                // Variables 1 §2), importance per declaration.
                let kept = property_dispatch::set_custom_source(
                    name,
                    decl.value,
                    Some(decl.text),
                    decl.important,
                    style,
                );
                if kept.is_err() {
                    warnings.push(invalid_value(&decl));
                }
                continue;
            }
            apply_declaration(decl, style, warnings);
        }
        if let Some(first) = self.first_warning {
            warnings[first..].sort_by_key(|w| (w.line, w.column));
        }
    }
}

/// A [`RawDeclaration`] that outlives its block's tokens.
struct OwnedDeclaration {
    name: String,
    value: Vec<Token>,
    text: String,
    important: bool,
    at: TokenPos,
}

#[derive(Debug)]
struct RawDeclaration<'a> {
    name: &'a str,
    value: &'a [Token],
    /// The value as written: the source from its first token to its
    /// last (`!important` excluded), whitespace and comments between
    /// them kept.
    text: &'a str,
    important: bool,
    /// Position of the property name in the source.
    at: TokenPos,
}

/// Split a token slice on top-level `;`s. Each non-empty segment
/// must contain `name : value …` — a leading ident followed by `:`.
/// A non-empty segment that doesn't match is dropped (CSS Syntax 3
/// §5.4.4) with a `MalformedDeclaration` warning; empty segments
/// (`;;`, trailing `;`) are silently fine.
/// A declaration block's source and, parallel to its tokens, their
/// positions and byte ranges in it.
struct Source<'a> {
    body: &'a str,
    positions: &'a [TokenPos],
    spans: &'a [rdom_style::parse::token::TokenSpan],
}

impl<'a> Source<'a> {
    /// The source of tokens `range` (empty for none).
    fn text(&self, range: std::ops::Range<usize>) -> &'a str {
        match (self.spans.get(range.start), range.end.checked_sub(1)) {
            (Some(first), Some(last)) if range.start < range.end => {
                &self.body[first.start..self.spans[last].end]
            }
            _ => "",
        }
    }
}

fn split_declarations<'a>(
    tokens: &'a [Token],
    source: &Source<'a>,
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
            let at = source.positions.get(start).copied().unwrap_or((0, 0));
            let text = |value: &[Token]| {
                // The value is the segment past `name :`, its first token
                // at `start + 2`.
                source.text(start + 2..start + 2 + value.len())
            };
            match into_declaration(segment, at, text) {
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

fn into_declaration<'a>(
    segment: &'a [Token],
    at: TokenPos,
    text: impl FnOnce(&[Token]) -> &'a str,
) -> Option<RawDeclaration<'a>> {
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
        text: text(value),
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

/// The warning for a declaration whose value its property rejects.
fn invalid_value(decl: &RawDeclaration) -> Warning {
    Warning {
        kind: WarningKind::InvalidValue {
            property: decl.name.to_string(),
            value: render_value(decl.value),
        },
        line: decl.at.0,
        column: decl.at.1,
    }
}

fn apply_declaration(decl: RawDeclaration, style: &mut TuiStyle, warnings: &mut Vec<Warning>) {
    let name = decl.name;
    let value = decl.value;
    let (line, column) = decl.at;

    // Single source of truth: rdom_style::property_dispatch owns
    // the name→setter table. The block parser is now a thin
    // tokenizer + per-declaration loop on top of that.
    match property_dispatch::set_from_source(name, value, Some(decl.text), decl.important, style) {
        Ok(()) => {}
        Err(DispatchError::UnknownProperty) => {
            warnings.push(Warning {
                kind: WarningKind::UnknownProperty(name.to_string()),
                line,
                column,
            });
        }
        // `DispatchError` is `#[non_exhaustive]`; an error added upstream
        // without a warning of its own reports as an invalid value (and
        // trips the assert in the workspace's tests until it gets one).
        Err(e) => {
            debug_assert!(
                matches!(e, DispatchError::InvalidValue),
                "unmapped dispatch error {e:?}"
            );
            warnings.push(invalid_value(&decl));
        }
    }
}
