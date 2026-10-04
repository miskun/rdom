//! `@scope` (CSS Cascade 6 §2.5).
//!
//! `@scope [(<scope-start>)]? [to (<scope-end>)]? { <block-contents> }`
//!
//! - `<scope-start>` is resolved against the nesting context: plain at
//!   the top level, relative to the parent style rule (`&`) inside one,
//!   relative to the enclosing scope's root inside another `@scope`;
//!   absent, the root is the parent of the sheet's owner node.
//! - `<scope-end>` and the body's style rules are relative to the
//!   scoping root (`rdom_core::selectors::parse_scoped`).
//! - The body's declarations apply to the root as `:where(:scope)`.
//!
//! An invalid prelude drops the rule, block included, with
//! `WarningKind::InvalidAtRulePrelude`.

use rdom_core::selectors::{self, SelectorList};
use rdom_style::parse::Cursor;
use rdom_style::{Scope, Stylesheet};

use crate::block::{Context, Parent, consume_scope_body};
use crate::layer::read_prelude;
use crate::top_level::skip_balanced_block;
use crate::{Warning, WarningKind};

/// Consume an `@scope` rule; the cursor is just past the at-keyword,
/// `at` is the position of `@`.
pub(crate) fn consume_scope_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: Context<'_>,
    at: (u32, u32),
) {
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    let scope = if cursor.peek() == Some('{') {
        boundaries(&prelude, ctx)
    } else {
        None
    };
    let Some((start, end)) = scope else {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: "scope".to_string(),
                prelude: prelude.trim().to_string(),
            },
            line: at.0,
            column: at.1,
        });
        match cursor.peek() {
            Some('{') => skip_balanced_block(cursor),
            Some(';') => {
                cursor.bump();
            }
            _ => {}
        }
        return;
    };
    let id = sheet.declare_scope(Scope::new(start, end, ctx.rule.scope));
    cursor.bump(); // '{'
    consume_scope_body(cursor, sheet, warnings, ctx.rule.in_scope(Some(id)));
}

/// The parsed `(<scope-start>)` / `to (<scope-end>)` of a prelude, or
/// `None` when it is invalid.
#[allow(clippy::type_complexity)]
fn boundaries(
    prelude: &str,
    ctx: Context<'_>,
) -> Option<(Option<SelectorList>, Option<SelectorList>)> {
    let mut rest = prelude.trim();
    let mut start = None;
    if rest.starts_with('(') {
        let (inner, after) = parenthesized(rest)?;
        start = Some(match ctx.parent {
            Parent::Top => selectors::parse(inner).ok()?,
            Parent::Rule(parent) => selectors::parse_nested(inner, &parent.nesting_list()).ok()?,
            Parent::Scope => selectors::parse_scoped(inner).ok()?,
        });
        rest = after.trim_start();
    }
    let mut end = None;
    if let Some(after) = strip_keyword(rest, "to") {
        let (inner, after) = parenthesized(after.trim_start())?;
        end = Some(selectors::parse_scoped(inner).ok()?);
        rest = after.trim_start();
    }
    rest.is_empty().then_some((start, end))
}

/// `to` followed by a non-identifier character.
fn strip_keyword<'s>(text: &'s str, keyword: &str) -> Option<&'s str> {
    let head = text.get(..keyword.len())?;
    let after = &text[keyword.len()..];
    let boundary = after
        .chars()
        .next()
        .is_some_and(|c| c.is_whitespace() || c == '(');
    (head.eq_ignore_ascii_case(keyword) && boundary).then_some(after)
}

/// Split `(…)…` at its balanced closing parenthesis (strings and
/// escapes respected): the inside and the rest.
fn parenthesized(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (_, '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth -= 1;
                if depth == 0 {
                    return Some((&text[1..i], &text[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}
