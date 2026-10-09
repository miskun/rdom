//! The conditional group rules (CSS Conditional 3 §2): `@media`
//! (Media Queries 4, Conditional 3 §3), `@supports` (Conditional 3 §6,
//! evaluated once, here, against what rdom parses) and `@container`
//! (Conditional 5 §6.4, evaluated per element by the backend).
//!
//! Each is declared in the sheet as a [`ConditionRule`] nested in the one
//! it sits in, and its body is parsed with the condition in the rule
//! context ([`RuleContext::in_condition`]) — at the top level and in
//! `@layer` as a list of rules, nested in a style rule as that rule's
//! block contents (CSS Nesting 1 §3.2). The condition is evaluated by the
//! backend's cascade, not here: a media query's answer changes with the
//! terminal.

use rdom_style::conditional::{ContainerQuery, MediaList, SupportsCondition};
use rdom_style::parse::SourceCursor;
use rdom_style::{ConditionKind, ConditionRule, RuleContext, Stylesheet};

use crate::{Warning, WarningKind};

/// After a conditional group rule's name (`@media`, `@supports`,
/// `@container`): read
/// the prelude and, at its `{` (consumed), declare the condition inside
/// `ctx`'s and return the context its body parses in. `None` when the
/// rule has no block or an invalid prelude (reported; the rule skipped
/// through its `;` or its block).
pub(crate) fn open_conditional_rule(
    name: &str,
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
    at: (u32, u32),
) -> Option<RuleContext> {
    let prelude = crate::layer::read_prelude(cursor, warnings)?;
    let kind = match cursor.peek() {
        Some('{') => condition_kind(name, prelude.trim()),
        _ => None,
    };
    let Some(kind) = kind else {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: name.to_ascii_lowercase(),
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
        return None;
    };
    cursor.bump(); // '{'
    Some(declare(sheet, ctx, kind))
}

/// Whether `name` is a conditional group rule this parser evaluates.
pub(crate) fn is_conditional(name: &str) -> bool {
    ["media", "supports", "container"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
}

/// The condition of `@name prelude`; `None` for a prelude its grammar
/// rejects (a media query list never is: an invalid query is `not all`).
fn condition_kind(name: &str, prelude: &str) -> Option<ConditionKind> {
    if name.eq_ignore_ascii_case("media") {
        Some(ConditionKind::Media(MediaList::parse(prelude)))
    } else if name.eq_ignore_ascii_case("supports") {
        SupportsCondition::parse(prelude).map(ConditionKind::Supports)
    } else if name.eq_ignore_ascii_case("container") {
        ContainerQuery::parse(prelude).map(ConditionKind::Container)
    } else {
        None
    }
}

/// Declare a condition of `kind` inside `ctx`'s and return the context
/// of its body.
pub(crate) fn declare(
    sheet: &mut Stylesheet,
    ctx: RuleContext,
    kind: ConditionKind,
) -> RuleContext {
    let id = sheet.declare_condition(ConditionRule::new(kind, ctx.condition));
    ctx.in_condition(Some(id))
}
