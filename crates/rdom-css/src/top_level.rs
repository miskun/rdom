//! Top-level stylesheet parse loop.
//!
//! Currently handles the structural shape — whitespace, comments,
//! and rule-block recognition (selector text up to `{`, ignored body
//! up to `}`). Declaration parsing arrives in §11.3.

use rdom_style::{Stylesheet, TuiStyle};

use crate::declarations;
use crate::{Warning, WarningKind};
use rdom_style::parse::Cursor;

/// Parse a full stylesheet. Mutates `sheet` in place via the
/// fluent-builder bridge (see `add_rule`).
pub(crate) fn parse_stylesheet(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
) {
    loop {
        if !skip_ws_and_comments(cursor, warnings) {
            return;
        }
        if cursor.is_eof() {
            return;
        }
        if !parse_one_rule(cursor, sheet, warnings) {
            return;
        }
    }
}

/// Skip whitespace and `/* … */` comments. Returns `false` if an
/// unterminated comment was hit (warning emitted, parse should
/// abort).
pub(crate) fn skip_ws_and_comments(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> bool {
    loop {
        match cursor.peek() {
            Some(c) if c.is_whitespace() => {
                cursor.bump();
            }
            Some('/') => {
                if let (_, Some('*')) = cursor.peek_two() {
                    if !skip_comment(cursor, warnings) {
                        return false;
                    }
                } else {
                    return true;
                }
            }
            _ => return true,
        }
    }
}

/// Consume a `/* … */` comment. The cursor is at `/`; we already
/// know the next char is `*`. Returns `false` on unterminated.
fn skip_comment(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> bool {
    let start_line = cursor.line();
    let start_col = cursor.col();
    cursor.bump(); // /
    cursor.bump(); // *
    loop {
        match cursor.bump() {
            None => {
                warnings.push(Warning {
                    kind: WarningKind::UnterminatedComment,
                    line: start_line,
                    column: start_col,
                });
                return false;
            }
            Some('*') => {
                if let Some('/') = cursor.peek() {
                    cursor.bump();
                    return true;
                }
            }
            Some(_) => {}
        }
    }
}

/// Parse one rule: `<selector> { <body> }`. Returns `false` if the
/// parse should abort (EOF in unexpected place, unterminated
/// comment).
fn parse_one_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
) -> bool {
    // Read selector text up to `{`, stripping comments inline.
    let selector = match read_selector_text(cursor, warnings) {
        Some(s) => s,
        None => return false,
    };
    if cursor.peek() != Some('{') {
        return false;
    }
    cursor.bump();
    // Capture body line/col for warning attribution before we read it.
    let body_line = cursor.line();
    let body_col = cursor.col();
    let body = match read_block_body(cursor, warnings) {
        Some(b) => b,
        None => return false,
    };
    // Parse declarations into a TuiStyle and a list of custom
    // properties. The selector decides what happens to the custom
    // properties: `:root` registers them in the stylesheet's
    // VarMap; other selectors drop them silently in M1 (per spec
    // §5.5).
    let mut style = TuiStyle::new();
    let mut custom_props: Vec<declarations::CustomProperty> = Vec::new();
    declarations::parse_block(
        &body,
        &mut style,
        &mut custom_props,
        body_line,
        body_col,
        warnings,
    );

    let trimmed = selector.trim();
    if trimmed == ":root" {
        // Register every captured custom property globally.
        // Stylesheet::define_var is fluent (consumes self), so we
        // mem-swap to mutate in place. Infallible — no recovery
        // needed.
        for cp in &custom_props {
            let owned = std::mem::take(sheet);
            *sheet = owned.define_var(&cp.name, &cp.value);
        }
    }

    if !trimmed.is_empty() && sheet.add_rule(trimmed, style).is_err() {
        warnings.push(Warning {
            kind: WarningKind::InvalidSelector(trimmed.to_string()),
            line: cursor.line(),
            column: cursor.col(),
        });
    }
    true
}

/// Read the selector text up to (but not consuming) `{`. Returns
/// `None` if EOF, unterminated comment, or no `{` found.
fn read_selector_text(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> Option<String> {
    let mut out = String::new();
    loop {
        match cursor.peek() {
            None => return None,
            Some('{') => return Some(out),
            Some('/') => {
                if let (_, Some('*')) = cursor.peek_two() {
                    if !skip_comment(cursor, warnings) {
                        return None;
                    }
                    // Insert a space so `a/* */b` -> `a b` (matches CSS).
                    if !out.ends_with(char::is_whitespace) && !out.is_empty() {
                        out.push(' ');
                    }
                } else {
                    out.push('/');
                    cursor.bump();
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// Read the body from the position just inside `{` up to (and
/// consuming) the matching `}`. Comments and string literals are
/// recognized so that `}` inside them doesn't terminate the body
/// early. The returned string is the verbatim body content
/// (comments preserved); `declarations::parse_block` re-tokenizes
/// it and skips comments there.
fn read_block_body(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> Option<String> {
    let mut out = String::new();
    loop {
        match cursor.peek() {
            None => return None,
            Some('}') => {
                cursor.bump();
                return Some(out);
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                // Preserve the comment in `out` so line/column
                // tracking inside the tokenizer stays accurate when
                // we add it later. Skip it here just to pass over
                // any embedded `}` inside.
                let start_line = cursor.line();
                let start_col = cursor.col();
                let comment_start = out.len();
                out.push('/');
                cursor.bump();
                out.push('*');
                cursor.bump();
                if !skip_comment_into(cursor, &mut out) {
                    out.truncate(comment_start);
                    warnings.push(Warning {
                        kind: WarningKind::UnterminatedComment,
                        line: start_line,
                        column: start_col,
                    });
                    return None;
                }
            }
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    // Unterminated string — let declarations::parse_block
                    // surface the warning when it tokenizes the body.
                    return Some(out);
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

fn skip_comment_into(cursor: &mut Cursor, out: &mut String) -> bool {
    loop {
        match cursor.peek() {
            None => return false,
            Some('*') => {
                out.push('*');
                cursor.bump();
                if cursor.peek() == Some('/') {
                    out.push('/');
                    cursor.bump();
                    return true;
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

fn read_string_into(cursor: &mut Cursor, quote: char, out: &mut String) -> bool {
    loop {
        match cursor.peek() {
            None => return false,
            Some(c) if c == quote => {
                out.push(c);
                cursor.bump();
                return true;
            }
            Some('\\') => {
                out.push('\\');
                cursor.bump();
                if let Some(esc) = cursor.peek() {
                    out.push(esc);
                    cursor.bump();
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}
