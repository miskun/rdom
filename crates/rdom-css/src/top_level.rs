//! Top-level stylesheet parse loop — CSS Syntax 3 §5.4 "consume a list
//! of rules" with its error recovery:
//!
//! - a qualified rule is a style rule, `<prelude> { <block> }`
//!   (`block.rs`, which also parses the rules nested in its block);
//! - `@import` is evaluated (`import.rs`) while it leads the sheet;
//! - `@layer` is evaluated (`layer.rs`); its block form parses a nested
//!   list of rules into the layer; `@scope` is evaluated (`scope.rs`),
//!   `@keyframes` too (`keyframes.rs`); `@media` and `@supports` declare
//!   their condition (`conditional.rs`) and parse their body as a list of
//!   rules under it;
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

use rdom_style::{LayerId, RuleContext, Stylesheet};

use crate::block::{Context, Parent, consume_style_rule};
use crate::import::Imports;
use crate::{Warning, WarningKind};
use rdom_style::parse::SourceCursor;

/// Parse a full stylesheet. Mutates `sheet` in place via the
/// fluent-builder bridge (see `add_rule`).
pub(crate) fn parse_stylesheet(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    imports: &mut Imports<'_>,
) {
    parse_rule_list(
        cursor,
        sheet,
        warnings,
        RuleContext::default(),
        Some(imports),
    );
}

/// §5.4.1 "consume a list of rules" in `ctx` — its layer (`None`:
/// unlayered) and whether it is `@starting-style`'s body.
/// `imports`: the list is a sheet's top level, where `@import` may
/// lead (CSS Cascade 5 §3); `None` for a block's body (`@layer x { … }`)
/// — its `}` ends it and is consumed, while at the top level a stray
/// `}` is dropped. EOF ends either (§5.4.7).
pub(crate) fn parse_rule_list(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
    imports: Option<&mut Imports<'_>>,
) {
    match imports {
        Some(imports) => rule_list(cursor, sheet, warnings, ctx, Some(imports)),
        None => in_block(cursor, warnings, |cursor, warnings| {
            rule_list(cursor, sheet, warnings, ctx, None);
        }),
    }
}

/// Run `body` over the block the cursor is just inside, one block
/// deeper — or, past [`MAX_BLOCK_DEPTH`](crate::MAX_BLOCK_DEPTH), skip
/// the block through its `}` without recursing, with
/// `WarningKind::BlockTooDeep` (C16G-DEPTH-CAPS).
pub(crate) fn in_block(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
    body: impl FnOnce(&mut SourceCursor, &mut Vec<Warning>),
) {
    if cursor.block_depth() >= crate::MAX_BLOCK_DEPTH {
        warnings.push(Warning {
            kind: WarningKind::BlockTooDeep,
            line: cursor.line(),
            column: cursor.col(),
        });
        crate::scan::skip_rest_of_block(cursor);
        return;
    }
    cursor.in_block(|cursor| body(cursor, warnings));
}

/// [`parse_rule_list`], within the nesting cap.
fn rule_list(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
    mut imports: Option<&mut Imports<'_>>,
) {
    let layer = ctx.layer;
    let nested = imports.is_none();
    // `@import` is valid only before every rule but `@charset` and
    // `@layer` statements.
    let mut imports_allowed = !nested;
    loop {
        if !skip_ws_and_comments(cursor, warnings) {
            return;
        }
        if cursor.peek() == Some('@') {
            match crate::import::leading_at_rule(cursor.rest()) {
                crate::import::Leading::Import => {
                    let allowed = imports_allowed;
                    crate::import::consume_import(
                        cursor,
                        sheet,
                        warnings,
                        layer,
                        imports.as_deref_mut().filter(|_| allowed),
                    );
                    continue;
                }
                crate::import::Leading::KeepsImports => {}
                crate::import::Leading::Other => imports_allowed = false,
            }
        } else if !matches!(cursor.peek(), None | Some('}')) {
            imports_allowed = false;
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
            Some('@') => consume_at_rule(cursor, sheet, warnings, ctx),
            Some(_) => {
                let ctx = Context {
                    rule: ctx,
                    parent: Parent::Top,
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
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
) {
    let layer = ctx.layer;
    let line = cursor.line();
    let column = cursor.col();
    cursor.bump(); // '@'
    let (name, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    match crate::at_rules::AtRule::of(&name) {
        Some(crate::at_rules::AtRule::Layer) => {
            let mut body = |cursor: &mut SourceCursor,
                            sheet: &mut Stylesheet,
                            warnings: &mut Vec<Warning>,
                            layer: Option<LayerId>| {
                parse_rule_list(cursor, sheet, warnings, ctx.in_layer(layer), None);
            };
            let place = (layer, ctx.condition);
            crate::layer::consume_layer_rule(
                cursor,
                sheet,
                warnings,
                place,
                (line, column),
                &mut body,
            );
            return;
        }
        Some(crate::at_rules::AtRule::Property) => {
            crate::property::consume_property_rule(
                cursor,
                sheet,
                warnings,
                ctx.condition,
                (line, column),
            );
            return;
        }
        Some(crate::at_rules::AtRule::Keyframes) => {
            crate::keyframes::consume_keyframes_rule(cursor, sheet, warnings, ctx, (line, column));
            return;
        }
        Some(crate::at_rules::AtRule::PositionTry) => {
            crate::position_try::consume_position_try_rule(
                cursor,
                sheet,
                warnings,
                ctx,
                (line, column),
            );
            return;
        }
        Some(crate::at_rules::AtRule::CounterStyle) => {
            crate::counter_style::consume_counter_style_rule(
                cursor,
                sheet,
                warnings,
                ctx,
                (line, column),
            );
            return;
        }
        Some(crate::at_rules::AtRule::Scope) => {
            let ctx = Context {
                rule: ctx,
                parent: Parent::Top,
            };
            crate::scope::consume_scope_rule(cursor, sheet, warnings, ctx, (line, column));
            return;
        }
        Some(crate::at_rules::AtRule::Conditional) => {
            // CSS Conditional 3 §3, §6: `@media <media-query-list> {
            // <rule-list> }`, `@supports <supports-condition> { … }`.
            if let Some(inner) = crate::conditional::open_conditional_rule(
                &name,
                cursor,
                sheet,
                warnings,
                ctx,
                (line, column),
            ) {
                parse_rule_list(cursor, sheet, warnings, inner, None);
            }
            return;
        }
        Some(crate::at_rules::AtRule::StartingStyle) => {
            // CSS Transitions 2 §3: `@starting-style { <rule-list> }`, no
            // prelude.
            if crate::block::starting_style_block(cursor, warnings, (line, column)) {
                parse_rule_list(cursor, sheet, warnings, ctx.in_starting_style(), None);
            }
            return;
        }
        None => {}
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
pub(crate) fn skip_at_rule_rest(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
    nested: bool,
) {
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
pub(crate) fn skip_balanced_block(cursor: &mut SourceCursor) {
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
pub(crate) fn skip_ws_and_comments(cursor: &mut SourceCursor, warnings: &mut Vec<Warning>) -> bool {
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
pub(crate) fn skip_comment(cursor: &mut SourceCursor, warnings: &mut Vec<Warning>) -> bool {
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

pub(crate) fn skip_comment_into(cursor: &mut SourceCursor, out: &mut String) -> bool {
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
pub(crate) fn copy_escape_into(cursor: &mut SourceCursor, out: &mut String) {
    out.push('\\');
    cursor.bump();
    if let Some(c) = cursor.bump() {
        out.push(c);
    }
}

pub(crate) fn read_string_into(cursor: &mut SourceCursor, quote: char, out: &mut String) -> bool {
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
