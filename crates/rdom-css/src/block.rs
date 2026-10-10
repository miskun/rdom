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
//!   parent; `@media` and `@supports` hold declarations and rules for the
//!   parent's elements under their condition (`conditional.rs`), and any other
//!   at-rule is reported and skipped.
//!
//! A nested rule whose prelude reaches `;` or the block's `}` before a
//! `{` is not a rule: it is reported as a malformed declaration (CSS
//! Syntax 3 parses it in that position as one).

use rdom_style::parse::SourceCursor;
use rdom_style::{LayerId, RuleContext, StyleSelector, Stylesheet, TuiStyle};

use crate::declarations::DeclarationRun;
use crate::scan::{is_declaration, read_declaration, read_prelude, skip_rest_of_block};
use crate::top_level::{skip_at_rule_rest, skip_ws_and_comments};
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
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: Context<'_>,
) -> bool {
    match consume_rule_head(cursor, warnings, ctx) {
        Head::End => false,
        Head::Dropped => true,
        Head::Rule(selector) => {
            let block = Block {
                selector: &selector,
                ctx: ctx.rule,
                children: Children::Nested,
            };
            consume_block_contents(cursor, sheet, warnings, block, true);
            true
        }
    }
}

/// A style rule's prelude, consumed through its `{`.
enum Head {
    /// A valid rule; the cursor is just inside its block.
    Rule(StyleSelector),
    /// No rule: an invalid selector (its block skipped, warned), or —
    /// nested — an item that ended at `;` / `}` before a block.
    Dropped,
    /// The input ended before a block.
    End,
}

fn consume_rule_head(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
    ctx: Context<'_>,
) -> Head {
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
        Ok(selector) if !text.is_empty() => Head::Rule(selector),
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
}

/// CSS Syntax 3 "consume a block's contents", from just inside `{`
/// through the closing `}` (or EOF, which closes the block, §5.4.7).
/// `own_rule`: the leading declarations are the rule itself, added
/// even when empty — a nested `@layer`'s block passes `false`, as all
/// its declarations are nested declarations rules.
fn consume_block_contents(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
    own_rule: bool,
) {
    crate::top_level::in_block(cursor, warnings, |cursor, warnings| {
        block_contents(cursor, sheet, warnings, block, own_rule);
    });
}

/// [`consume_block_contents`] within the nesting cap.
fn block_contents(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
    own_rule: bool,
) {
    let mut run: Option<DeclarationRun> = None;
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
                    flush(sheet, warnings, block, &mut run, &mut pending_own);
                }
                consume_nested_at_rule(cursor, sheet, warnings, block);
            }
            Some(_) if is_declaration(cursor.rest()) => {
                let at = (cursor.line(), cursor.col());
                let text = read_declaration(cursor);
                run.get_or_insert_with(DeclarationRun::default)
                    .push(&text, at.0, at.1, warnings);
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
                    Head::Rule(selector) => {
                        // Only a rule that parses ends the run of
                        // declarations before it.
                        flush(sheet, warnings, block, &mut run, &mut pending_own);
                        let child = Block {
                            selector: &selector,
                            ctx: block.ctx,
                            children: Children::Nested,
                        };
                        consume_block_contents(cursor, sheet, warnings, child, true);
                    }
                }
            }
        }
    }
    flush(sheet, warnings, block, &mut run, &mut pending_own);
}

/// Add the declarations collected since the last nested rule as a rule
/// with the block's selector: the rule's own the first time
/// (`pending_own`, even if empty), a nested declarations rule after.
fn flush(
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
    run: &mut Option<DeclarationRun>,
    pending_own: &mut bool,
) {
    let mut style = TuiStyle::new();
    match run.take() {
        Some(run) => run.apply(&mut style, warnings),
        None if *pending_own => {}
        None => return,
    }
    *pending_own = false;
    sheet.add_style_rule(block.selector, style, block.ctx);
}

/// An at-rule inside a style rule's block (CSS Nesting 1 §3.2).
fn consume_nested_at_rule(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    block: Block<'_>,
) {
    let at = (cursor.line(), cursor.col());
    cursor.bump(); // '@'
    let (name, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    if name.eq_ignore_ascii_case("layer") {
        let mut body = |cursor: &mut SourceCursor,
                        sheet: &mut Stylesheet,
                        warnings: &mut Vec<Warning>,
                        layer: Option<LayerId>| {
            let inner = Block {
                ctx: block.ctx.in_layer(layer),
                ..block
            };
            consume_block_contents(cursor, sheet, warnings, inner, false);
        };
        let place = (block.ctx.layer, block.ctx.condition);
        crate::layer::consume_layer_rule(cursor, sheet, warnings, place, at, &mut body);
        return;
    }
    if crate::conditional::is_conditional(&name) {
        // CSS Conditional 3 §3, §6, nested (CSS Nesting 1 §3.2): the
        // block's declarations and rules are the parent rule's, under the
        // condition.
        if let Some(ctx) =
            crate::conditional::open_conditional_rule(&name, cursor, sheet, warnings, block.ctx, at)
        {
            let inner = Block { ctx, ..block };
            consume_block_contents(cursor, sheet, warnings, inner, false);
        }
        return;
    }
    if name.eq_ignore_ascii_case("starting-style") {
        // CSS Transitions 2 §3, nested (CSS Nesting 1 §3.2): its block's
        // declarations and rules are the parent rule's, starting style
        // only.
        if starting_style_block(cursor, warnings, at) {
            let inner = Block {
                ctx: block.ctx.in_starting_style(),
                ..block
            };
            consume_block_contents(cursor, sheet, warnings, inner, false);
        }
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
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
) {
    let root = StyleSelector::parse_scoped("&").expect("`&` parses as a scoped selector");
    let block = Block {
        selector: &root,
        ctx,
        children: Children::Scoped,
    };
    consume_block_contents(cursor, sheet, warnings, block, false);
}

/// The at-rules a style rule's block evaluates (CSS Nesting 1 §3.2).
fn is_evaluated_nested_at_rule(name: &str) -> bool {
    crate::conditional::is_conditional(name)
        || ["layer", "scope", "starting-style"]
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n))
}

/// After `@starting-style`'s name: an empty prelude then `{` — consumed,
/// `true` — or an invalid rule, reported and skipped (`false`).
pub(crate) fn starting_style_block(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
    at: (u32, u32),
) -> bool {
    let Some(prelude) = crate::layer::read_prelude(cursor, warnings) else {
        return false;
    };
    if prelude.trim().is_empty() && cursor.peek() == Some('{') {
        cursor.bump();
        return true;
    }
    warnings.push(Warning {
        kind: WarningKind::InvalidAtRulePrelude {
            name: "starting-style".to_string(),
            prelude: prelude.trim().to_string(),
        },
        line: at.0,
        column: at.1,
    });
    match cursor.peek() {
        Some('{') => crate::top_level::skip_balanced_block(cursor),
        Some(';') => {
            cursor.bump();
        }
        _ => {}
    }
    false
}
