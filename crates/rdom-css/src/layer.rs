//! `@layer` (CSS Cascade 5 §6.4.1).
//!
//! - Statement form, `@layer a, b.c;`: declares each named layer, in
//!   order, which fixes its place among its siblings.
//! - Block form, `@layer a { … }` / `@layer { … }`: declares the named
//!   layer (or a new anonymous one) and parses the block's rules into
//!   it; an `@layer` inside the block nests under it.
//!
//! A `<layer-name>` is `<ident> [ '.' <ident> ]*` with no whitespace
//! around the dots; the CSS-wide keywords are reserved and make the
//! rule invalid. An invalid prelude drops the whole rule (block
//! included) with `WarningKind::InvalidAtRulePrelude`.

use rdom_style::parse::Cursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::{LayerId, Stylesheet};

use crate::top_level::{parse_rule_list, read_string_into, skip_balanced_block, skip_comment};
use crate::{Warning, WarningKind};

/// Consume an `@layer` rule; the cursor is just past the at-keyword.
/// `parent` is the layer the rule sits in; `at` the position of `@`.
pub(crate) fn consume_layer_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    parent: Option<LayerId>,
    at: (u32, u32),
) {
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    let names = layer_names(&prelude);
    let invalid = |warnings: &mut Vec<Warning>| {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: "layer".to_string(),
                prelude: prelude.trim().to_string(),
            },
            line: at.0,
            column: at.1,
        });
    };
    if cursor.peek() == Some('{') {
        let layer = match names.as_deref() {
            Some([]) => Some(sheet.declare_anonymous_layer(parent)),
            Some([name]) => sheet.declare_layer(parent, &segments(name)),
            _ => {
                invalid(warnings);
                skip_balanced_block(cursor);
                return;
            }
        };
        cursor.bump(); // '{'
        parse_rule_list(cursor, sheet, warnings, layer, true);
        return;
    }
    // `;`, or EOF (which ends the statement, CSS Syntax 3 §5.4.2).
    if cursor.peek() == Some(';') {
        cursor.bump();
    }
    match names {
        Some(list) if !list.is_empty() => {
            for name in &list {
                sheet.declare_layer(parent, &segments(name));
            }
        }
        _ => invalid(warnings),
    }
}

fn segments(name: &[String]) -> Vec<&str> {
    name.iter().map(String::as_str).collect()
}

/// The prelude up to (not including) `;` or `{`, comments removed and
/// strings kept. `None` on an unterminated comment (warned).
fn read_prelude(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> Option<String> {
    let mut out = String::new();
    loop {
        match cursor.peek() {
            None | Some(';' | '{') => return Some(out),
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return Some(out);
                }
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                if !skip_comment(cursor, warnings) {
                    return None;
                }
                out.push(' ');
            }
            Some('\\') => {
                out.push('\\');
                cursor.bump();
                if let Some(c) = cursor.bump() {
                    out.push(c);
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// The comma-separated `<layer-name>` list of a prelude, each name as
/// its segments; `Some(vec![])` for an empty prelude, `None` when it is
/// not a valid list.
fn layer_names(prelude: &str) -> Option<Vec<Vec<String>>> {
    // No whitespace may surround a `.` (escaped dots are inside an
    // ident and decode there).
    let chars: Vec<char> = prelude.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let escaped = i > 0 && chars[i - 1] == '\\';
        if c == '.' && !escaped {
            let before = i.checked_sub(1).map(|j| chars[j]);
            let after = chars.get(i + 1).copied();
            if before.is_none_or(char::is_whitespace) || after.is_none_or(char::is_whitespace) {
                return None;
            }
        }
    }
    let tokens = tokenize(prelude).ok()?;
    let mut names = Vec::new();
    for item in tokens.split(|t| *t == Token::Comma) {
        names.push(layer_name(item)?);
    }
    if names.len() == 1 && names[0].is_empty() {
        return Some(Vec::new());
    }
    names.iter().all(|n| !n.is_empty()).then_some(names)
}

/// `<ident> [ '.' <ident> ]*`; empty for an empty token list.
fn layer_name(tokens: &[Token]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match (i % 2, token) {
            (0, Token::Ident(s)) if !is_reserved(s) => out.push(s.clone()),
            (1, Token::Delim('.')) => {}
            _ => return None,
        }
    }
    (tokens.len() % 2 == 1 || tokens.is_empty()).then_some(out)
}

/// The CSS-wide keywords are reserved as layer names (Cascade 5 §6.4.1).
fn is_reserved(name: &str) -> bool {
    [
        "initial",
        "inherit",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ]
    .iter()
    .any(|k| name.eq_ignore_ascii_case(k))
}
