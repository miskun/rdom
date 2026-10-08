//! The conditional group rules (CSS Conditional 3 §2): `@media`
//! (Media Queries 4, Conditional 3 §3).
//!
//! Each is declared in the sheet as a [`ConditionRule`] nested in the one
//! it sits in, and its body is parsed with the condition in the rule
//! context ([`RuleContext::in_condition`]) — at the top level and in
//! `@layer` as a list of rules, nested in a style rule as that rule's
//! block contents (CSS Nesting 1 §3.2). The condition is evaluated by the
//! backend's cascade, not here: a media query's answer changes with the
//! terminal.

use rdom_style::conditional::MediaList;
use rdom_style::parse::SourceCursor;
use rdom_style::{ConditionKind, ConditionRule, RuleContext, Stylesheet};

use crate::{Warning, WarningKind};

/// After `@media`'s name: read the prelude and, at its `{` (consumed),
/// declare the condition inside `ctx`'s and return the context its body
/// parses in. `None` when the rule has no block (reported, and skipped
/// through its `;`).
pub(crate) fn open_media_rule(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
    at: (u32, u32),
) -> Option<RuleContext> {
    let prelude = crate::layer::read_prelude(cursor, warnings)?;
    if cursor.peek() != Some('{') {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: "media".to_string(),
                prelude: prelude.trim().to_string(),
            },
            line: at.0,
            column: at.1,
        });
        if cursor.peek() == Some(';') {
            cursor.bump();
        }
        return None;
    }
    cursor.bump(); // '{'
    let queries = MediaList::parse(prelude.trim());
    Some(declare(sheet, ctx, ConditionKind::Media(queries)))
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
