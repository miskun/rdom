//! Top-level stylesheet parse loop — CSS Syntax 3 §5.4 "consume a list
//! of rules" with its error recovery:
//!
//! - a qualified rule is a style rule, `<prelude> { <block> }`
//!   (`block.rs`, which also parses the rules nested in its block);
//! - `@layer` is evaluated (`layer.rs`); its block form parses a nested
//!   list of rules into the layer;
//! - any other at-rule (`@name …`) is consumed whole — statement form
//!   through `;`, block form through a depth-tracked `{…}` — and
//!   reported as `UnsupportedAtRule`;
//! - a stray `}` at the top level is a parse error and is ignored
//!   (§5.4.1);
//! - EOF inside a block closes the block and keeps the rule (§5.4.7).
//!
//! Without the at-rule and stray-brace paths, `@import …;` and
//! `@media {…}` were read as the *next* rule's selector text and
//! swallowed that rule.

use rdom_style::{LayerId, Stylesheet};

use crate::block::{Context, consume_style_rule};
use crate::{Warning, WarningKind};
use rdom_style::parse::Cursor;

/// Parse a full stylesheet. Mutates `sheet` in place via the
/// fluent-builder bridge (see `add_rule`).
pub(crate) fn parse_stylesheet(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
) {
    parse_rule_list(cursor, sheet, warnings, None, false);
}

/// §5.4.1 "consume a list of rules" into `layer` (`None`: unlayered).
/// `nested`: the list is a block's body (`@layer x { … }`) — its `}`
/// ends it and is consumed; at the top level a stray `}` is dropped.
/// EOF ends either (§5.4.7).
pub(crate) fn parse_rule_list(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    layer: Option<LayerId>,
    nested: bool,
) {
    loop {
        if !skip_ws_and_comments(cursor, warnings) {
            return;
        }
        match cursor.peek() {
            None => return,
            // §5.4.1: a `}` at the top level is a parse error; drop it
            // and carry on with the next rule.
            Some('}') => {
                cursor.bump();
                if nested {
                    return;
                }
            }
            Some('@') => consume_at_rule(cursor, sheet, warnings, layer),
            Some(_) => {
                let ctx = Context {
                    layer,
                    parent: None,
                };
                if !consume_style_rule(cursor, sheet, warnings, ctx) {
                    return;
                }
            }
        }
    }
}

/// Consume one at-rule starting at `@` (§5.4.2 "consume an at-rule"):
/// the name, then the prelude up to either `;` (statement at-rule,
/// e.g. `@import`, `@charset`) or a `{…}` block (e.g. `@media`,
/// `@keyframes`, `@font-face`), whose nested blocks are skipped by
/// depth. `@layer` goes to `layer.rs`; any other at-rule emits
/// `UnsupportedAtRule(name)` positioned at the `@`. At-rule names are
/// ASCII case-insensitive.
fn consume_at_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    layer: Option<LayerId>,
) {
    let line = cursor.line();
    let column = cursor.col();
    cursor.bump(); // '@'
    let (name, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    if name.eq_ignore_ascii_case("layer") {
        let mut body = |cursor: &mut Cursor,
                        sheet: &mut Stylesheet,
                        warnings: &mut Vec<Warning>,
                        layer: Option<LayerId>| {
            parse_rule_list(cursor, sheet, warnings, layer, true);
        };
        crate::layer::consume_layer_rule(cursor, sheet, warnings, layer, (line, column), &mut body);
        return;
    }
    warnings.push(Warning {
        kind: WarningKind::UnsupportedAtRule(name),
        line,
        column,
    });
    skip_at_rule_rest(cursor, warnings, false);
}

/// Skip the rest of an at-rule the parser does not evaluate: its
/// prelude through `;` (statement at-rule, e.g. `@charset`) or through
/// its `{…}` block, nested blocks skipped by depth. Strings and
/// comments in the prelude may hold `;` / `{`. `nested`: the at-rule
/// sits in a style rule's block, whose `}` also ends a statement (and
/// is left for the block).
pub(crate) fn skip_at_rule_rest(cursor: &mut Cursor, warnings: &mut Vec<Warning>, nested: bool) {
    loop {
        match cursor.peek() {
            None => return,
            Some(';') => {
                cursor.bump();
                return;
            }
            Some('}') if nested => return,
            Some('{') => {
                skip_balanced_block(cursor);
                return;
            }
            Some(q @ ('"' | '\'')) => {
                cursor.bump();
                let mut sink = String::new();
                if !read_string_into(cursor, q, &mut sink) {
                    return; // EOF inside the string: at-rule ends with input
                }
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                if !skip_comment(cursor, warnings) {
                    return;
                }
            }
            Some(_) => {
                cursor.bump();
            }
        }
    }
}

/// With the cursor on `{`, consume through the matching `}` (nested
/// blocks, strings, and comments respected). At EOF the block is
/// treated as closed (§5.4.7).
pub(crate) fn skip_balanced_block(cursor: &mut Cursor) {
    let mut depth = 0usize;
    loop {
        match cursor.peek() {
            None => return,
            Some('{') => {
                depth += 1;
                cursor.bump();
            }
            Some('}') => {
                cursor.bump();
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return;
                }
            }
            Some(q @ ('"' | '\'')) => {
                cursor.bump();
                let mut sink = String::new();
                if !read_string_into(cursor, q, &mut sink) {
                    return;
                }
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                let mut sink = String::new();
                cursor.bump();
                cursor.bump();
                if !skip_comment_into(cursor, &mut sink) {
                    return;
                }
            }
            Some(_) => {
                cursor.bump();
            }
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
pub(crate) fn skip_comment(cursor: &mut Cursor, warnings: &mut Vec<Warning>) -> bool {
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

pub(crate) fn skip_comment_into(cursor: &mut Cursor, out: &mut String) -> bool {
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

/// With the cursor on `\`, copy the backslash and the code point it
/// escapes verbatim, so an escaped `{`, `}`, quote or `,` is never read
/// as structure. Decoding is the consumer's job.
pub(crate) fn copy_escape_into(cursor: &mut Cursor, out: &mut String) {
    out.push('\\');
    cursor.bump();
    if let Some(c) = cursor.bump() {
        out.push(c);
    }
}

pub(crate) fn read_string_into(cursor: &mut Cursor, quote: char, out: &mut String) -> bool {
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
