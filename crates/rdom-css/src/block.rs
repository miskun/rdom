//! Style rules and their blocks — CSS Syntax 3 "consume a qualified
//! rule" / "consume a block's contents", with CSS Nesting 1.
//!
//! A style rule's block holds declarations, nested style rules and
//! nested at-rules, interleaved (Nesting 1 §2–§3):
//!
//! - an item that starts `<ident> :` and has no top-level `{}` block
//!   before its `;` / `}` is a declaration (a custom property may hold
//!   a block); anything else is a nested style rule, whose selector is
//!   resolved against the parent's (`StyleSelector::parse_nested`);
//! - the declarations before the first nested rule are the rule's own;
//!   each later run of declarations is a *nested declarations rule*
//!   (`CSSNestedDeclarations`) with the parent's selector, placed after
//!   the nested rules before it in order of appearance;
//! - a nested at-rule goes to [`consume_nested_at_rule`]: `@layer`
//!   holds declarations and rules for the parent's elements, in the
//!   layer; `@scope` (`scope.rs`) takes its start relative to the
//!   parent; the conditional group rules (`@media`, `@supports`,
//!   `@container`) plug in there when they land (C14), and any other
//!   at-rule is reported and skipped.
//!
//! A nested rule whose prelude reaches `;` or the block's `}` before a
//! `{` is not a rule: it is reported as a malformed declaration (CSS
//! Syntax 3 parses it in that position as one).

use rdom_style::parse::Cursor;
use rdom_style::{LayerId, RuleContext, StyleSelector, Stylesheet, TuiStyle};

use crate::declarations;
use crate::top_level::{
    copy_escape_into, read_string_into, skip_at_rule_rest, skip_comment, skip_comment_into,
    skip_ws_and_comments,
};
use crate::{Warning, WarningKind};

/// Where a style rule sits: its cascade layer and scope, and what its
/// selector is relative to.
#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    pub rule: RuleContext,
    pub parent: Parent<'a>,
}

/// What a style rule's selector is relative to.
#[derive(Clone, Copy)]
pub(crate) enum Parent<'a> {
    /// Nothing: a top-level rule.
    Top,
    /// The style rule it is nested in (CSS Nesting 1 §2).
    Rule(&'a StyleSelector),
    /// The scoping root: a rule directly in `@scope` (CSS Cascade 6
    /// §2.5.2).
    Scope,
}

impl Context<'_> {
    fn nested(&self) -> bool {
        !matches!(self.parent, Parent::Top)
    }
}

/// Consume one style rule, from its prelude through its block.
/// Returns `false` when the input ended before a block (the caller
/// stops).
pub(crate) fn consume_style_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: Context<'_>,
) -> bool {
    match consume_rule_head(cursor, warnings, ctx) {
        Head::End => false,
        Head::Dropped => true,
        Head::Rule(selector, root_vars) => {
            let block = Block {
                selector: &selector,
                ctx: ctx.rule,
                root_vars,
                children: Children::Nested,
            };
            consume_block_contents(cursor, sheet, warnings, block, true);
            true
        }
    }
}

/// A style rule's prelude, consumed through its `{`.
enum Head {
    /// A valid rule; the cursor is just inside its block. The flag
    /// marks a top-level `:root` rule (`Block::root_vars`).
    Rule(StyleSelector, bool),
    /// No rule: an invalid selector (its block skipped, warned), or —
    /// nested — an item that ended at `;` / `}` before a block.
    Dropped,
    /// The input ended before a block.
    End,
}

fn consume_rule_head(cursor: &mut Cursor, warnings: &mut Vec<Warning>, ctx: Context<'_>) -> Head {
    let at = (cursor.line(), cursor.col());
    let nested = ctx.nested();
    let Some(prelude) = read_prelude(cursor, warnings, nested) else {
        return Head::End;
    };
    match cursor.peek() {
        Some('{') => {}
        // Nested: a `;` or the parent's `}` ends a would-be rule with
        // no block — CSS Syntax 3 reads that item as a declaration.
        Some(c @ (';' | '}')) if nested => {
            if c == ';' {
                cursor.bump();
            }
            warnings.push(Warning {
                kind: WarningKind::MalformedDeclaration(prelude.trim().to_string()),
                line: at.0,
                column: at.1,
            });
            return Head::Dropped;
        }
        _ => return Head::End,
    }
    cursor.bump(); // '{'
    let text = prelude.trim();
    let selector = match ctx.parent {
        Parent::Top => StyleSelector::parse(text),
        Parent::Rule(parent) => StyleSelector::parse_nested(text, parent),
        Parent::Scope => StyleSelector::parse_scoped(text),
    };
    match selector {
        Ok(selector) if !text.is_empty() => {
            Head::Rule(selector, !nested && text.eq_ignore_ascii_case(":root"))
        }
        result => {
            if result.is_err() && !text.is_empty() {
                warnings.push(Warning {
                    kind: WarningKind::InvalidSelector(text.to_string()),
                    line: at.0,
                    column: at.1,
                });
            }
            skip_rest_of_block(cursor);
            Head::Dropped
        }
    }
}

/// The rule a block's declarations belong to.
#[derive(Clone, Copy)]
struct Block<'a> {
    selector: &'a StyleSelector,
    ctx: RuleContext,
    /// How the block's nested style rules parse their selectors.
    children: Children,
    /// A top-level `:root` rule: its custom properties are also the
    /// sheet's root variables (`Stylesheet::vars`).
    root_vars: bool,
}

/// CSS Syntax 3 "consume a block's contents", from just inside `{`
/// through the closing `}` (or EOF, which closes the block, §5.4.7).
/// `own_rule`: the leading declarations are the rule itself, added
/// even when empty — a nested `@layer`'s block passes `false`, as all
/// its declarations are nested declarations rules.
fn consume_block_contents(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
    own_rule: bool,
) {
    let mut run: Option<TuiStyle> = None;
    let mut pending_own = own_rule;
    loop {
        if !skip_ws_and_comments(cursor, warnings) {
            break;
        }
        match cursor.peek() {
            None => break,
            Some('}') => {
                cursor.bump();
                break;
            }
            Some(';') => {
                cursor.bump();
            }
            Some('@') => {
                // An at-rule that becomes a rule (`@layer`) ends the run
                // of declarations before it; a dropped one does not.
                let name = rdom_core::css_syntax::consume_ident(&cursor.rest()[1..]).0;
                if is_evaluated_nested_at_rule(&name) {
                    flush(sheet, block, &mut run, &mut pending_own);
                }
                consume_nested_at_rule(cursor, sheet, warnings, block);
            }
            Some(_) if is_declaration(cursor.rest()) => {
                let at = (cursor.line(), cursor.col());
                let text = read_declaration(cursor);
                let style = run.get_or_insert_with(TuiStyle::new);
                declarations::parse_block(&text, style, at.0, at.1, warnings);
            }
            Some(_) => {
                let ctx = Context {
                    rule: block.ctx,
                    parent: match block.children {
                        Children::Nested => Parent::Rule(block.selector),
                        Children::Scoped => Parent::Scope,
                    },
                };
                match consume_rule_head(cursor, warnings, ctx) {
                    Head::End => break,
                    Head::Dropped => {}
                    Head::Rule(selector, _) => {
                        // Only a rule that parses ends the run of
                        // declarations before it.
                        flush(sheet, block, &mut run, &mut pending_own);
                        let child = Block {
                            selector: &selector,
                            ctx: block.ctx,
                            root_vars: false,
                            children: Children::Nested,
                        };
                        consume_block_contents(cursor, sheet, warnings, child, true);
                    }
                }
            }
        }
    }
    flush(sheet, block, &mut run, &mut pending_own);
}

/// Add the declarations collected since the last nested rule as a rule
/// with the block's selector: the rule's own the first time
/// (`pending_own`, even if empty), a nested declarations rule after.
fn flush(
    sheet: &mut Stylesheet,
    block: Block<'_>,
    run: &mut Option<TuiStyle>,
    pending_own: &mut bool,
) {
    let style = match run.take() {
        Some(style) => style,
        None if *pending_own => TuiStyle::new(),
        None => return,
    };
    *pending_own = false;
    if block.root_vars {
        for d in &style.custom_properties {
            sheet.define_var_mut(&d.name, &d.value);
        }
    }
    sheet.add_style_rule(block.selector, style, block.ctx);
}

/// An at-rule inside a style rule's block (CSS Nesting 1 §3.2).
fn consume_nested_at_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
) {
    let at = (cursor.line(), cursor.col());
    cursor.bump(); // '@'
    let (name, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    if name.eq_ignore_ascii_case("layer") {
        let mut body = |cursor: &mut Cursor,
                        sheet: &mut Stylesheet,
                        warnings: &mut Vec<Warning>,
                        layer: Option<LayerId>| {
            let inner = Block {
                ctx: block.ctx.in_layer(layer),
                root_vars: false,
                ..block
            };
            consume_block_contents(cursor, sheet, warnings, inner, false);
        };
        let layer = block.ctx.layer;
        crate::layer::consume_layer_rule(cursor, sheet, warnings, layer, at, &mut body);
        return;
    }
    if name.eq_ignore_ascii_case("scope") {
        let ctx = Context {
            rule: block.ctx,
            parent: match block.children {
                Children::Nested => Parent::Rule(block.selector),
                Children::Scoped => Parent::Scope,
            },
        };
        crate::scope::consume_scope_rule(cursor, sheet, warnings, ctx, at);
        return;
    }
    warnings.push(Warning {
        kind: WarningKind::UnsupportedAtRule(name),
        line: at.0,
        column: at.1,
    });
    skip_at_rule_rest(cursor, warnings, true);
}

/// How a block's nested style rules parse their selectors.
#[derive(Clone, Copy)]
enum Children {
    /// Relative to the block's rule (CSS Nesting 1 §2).
    Nested,
    /// Relative to the scoping root: `@scope`'s body (CSS Cascade 6
    /// §2.5.2).
    Scoped,
}

/// The body of an `@scope` rule (CSS Cascade 6 §2.5.2), from just
/// inside its `{`: scoped style rules, and declarations that apply to
/// the scoping root as `:where(:scope)` (zero specificity).
pub(crate) fn consume_scope_body(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
) {
    let root = StyleSelector::parse_scoped("&").expect("`&` parses as a scoped selector");
    let block = Block {
        selector: &root,
        ctx,
        root_vars: false,
        children: Children::Scoped,
    };
    consume_block_contents(cursor, sheet, warnings, block, false);
}

/// The at-rules a style rule's block evaluates (CSS Nesting 1 §3.2).
/// The conditional group rules join here when they land (C14).
fn is_evaluated_nested_at_rule(name: &str) -> bool {
    ["layer", "scope"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
}

/// Does the block item at the start of `rest` parse as a declaration?
/// `<ident> <ws>* :`, then — unless the name is a custom property's —
/// no top-level `{` before the `;` or `}` that ends it.
fn is_declaration(rest: &str) -> bool {
    if !rdom_core::css_syntax::would_start_ident(rest) {
        return false;
    }
    let (name, used) = rdom_core::css_syntax::consume_ident(rest);
    let after = rest[used..].trim_start();
    if !after.starts_with(':') {
        return false;
    }
    if name.starts_with("--") {
        return true;
    }
    let mut chars = after[1..].chars().peekable();
    let mut depth = 0usize;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            q @ ('"' | '\'') => {
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        chars.next();
                    } else if c == q {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut star = false;
                for c in chars.by_ref() {
                    if star && c == '/' {
                        break;
                    }
                    star = c == '*';
                }
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '{' if depth == 0 => return false,
            ';' | '}' if depth == 0 => return true,
            _ => {}
        }
    }
    true
}

/// Consume one declaration's text through its `;` (consumed) or up to
/// the block's `}` (not consumed) or EOF. Strings, comments, escapes
/// and bracketed groups are passed through whole.
fn read_declaration(cursor: &mut Cursor) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    loop {
        match cursor.peek() {
            None => return out,
            Some(';') if depth == 0 => {
                cursor.bump();
                return out;
            }
            Some('}') if depth == 0 => return out,
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return out;
                }
            }
            Some('\\') => copy_escape_into(cursor, &mut out),
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                out.push_str("/*");
                cursor.bump();
                cursor.bump();
                if !skip_comment_into(cursor, &mut out) {
                    return out;
                }
            }
            Some(c) => {
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// A style rule's prelude up to (not consuming) `{` — or, nested, up
/// to a `;` / `}` that ends the item first. Comments become a space;
/// strings and escapes are copied through so a `{` in them does not
/// end the prelude. `None` at EOF or on an unterminated comment.
fn read_prelude(cursor: &mut Cursor, warnings: &mut Vec<Warning>, nested: bool) -> Option<String> {
    let mut out = String::new();
    loop {
        match cursor.peek() {
            None => return None,
            Some('{') => return Some(out),
            Some(';' | '}') if nested => return Some(out),
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return None;
                }
            }
            // An escape (CSS Syntax 3 §4.3.7) is copied through
            // undecoded — the selector parser decodes it — so `.x\{`
            // does not end the prelude.
            Some('\\') => copy_escape_into(cursor, &mut out),
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                if !skip_comment(cursor, warnings) {
                    return None;
                }
                // `a/* */b` is `a b`.
                if !out.ends_with(char::is_whitespace) && !out.is_empty() {
                    out.push(' ');
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// From just inside a block's `{`, skip through its matching `}`.
fn skip_rest_of_block(cursor: &mut Cursor) {
    let mut depth = 1usize;
    loop {
        match cursor.peek() {
            None => return,
            Some('{') => {
                depth += 1;
                cursor.bump();
            }
            Some('}') => {
                cursor.bump();
                depth -= 1;
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
            Some('\\') => {
                let mut sink = String::new();
                copy_escape_into(cursor, &mut sink);
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
