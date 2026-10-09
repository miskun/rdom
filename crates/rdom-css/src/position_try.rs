//! `@position-try` (CSS Anchor Positioning 1 §4.1).
//!
//! `@position-try --above { bottom: anchor(top); top: auto }` defines the
//! position option `--above` in the sheet (`Stylesheet::define_position_try`),
//! in the cascade layer and conditional group rule it sits in. The
//! prelude is one `<dashed-ident>`; an invalid one drops the rule
//! (`WarningKind::InvalidAtRulePrelude`). The body takes the inset,
//! margin, sizing and self-alignment properties, `position-anchor` and
//! `position-area` (`PositionTryRule::accepts`): any other declaration
//! is dropped (`WarningKind::PositionTryDescriptorDropped`, reason
//! `NotADescriptor`), and so is an `!important` one (`Important` — §4.1:
//! the descriptors take no `!important`).

use rdom_style::parse::SourceCursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::{PositionTryRule, RuleContext, Stylesheet, TuiStyle};

use crate::declarations::DeclarationRun;
use crate::layer::read_prelude;
use crate::property::read_body;
use crate::top_level::skip_at_rule_rest;
use crate::{PositionTryDescriptorReason, Warning, WarningKind};

/// Consume a `@position-try` rule; the cursor is just past the
/// at-keyword, `at` is the position of `@`, `ctx` the cascade layer and
/// the conditional group rule it sits in.
pub(crate) fn consume_position_try_rule(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    ctx: RuleContext,
    at: (u32, u32),
) {
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    let name = match tokenize(prelude.trim()).as_deref() {
        Ok([Token::Ident(s)]) if s.starts_with("--") && s.len() > 2 => Some(s.clone()),
        _ => None,
    };
    let has_block = cursor.peek() == Some('{');
    let Some(name) = name.filter(|_| has_block) else {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: "position-try".to_string(),
                prelude: prelude.trim().to_string(),
            },
            line: at.0,
            column: at.1,
        });
        skip_at_rule_rest(cursor, warnings, false);
        return;
    };
    cursor.bump(); // '{'
    let (line, column) = (cursor.line(), cursor.col());
    let body = read_body(cursor).unwrap_or_default();
    let mut run = DeclarationRun::default();
    run.push(&body, line, column, warnings);
    run.retain(warnings, |descriptor, important| {
        let reason = if !PositionTryRule::accepts(descriptor) {
            PositionTryDescriptorReason::NotADescriptor
        } else if important {
            PositionTryDescriptorReason::Important
        } else {
            return None;
        };
        Some(WarningKind::PositionTryDescriptorDropped {
            name: name.to_string(),
            descriptor: descriptor.to_ascii_lowercase(),
            reason,
        })
    });
    let mut style = TuiStyle::new();
    run.apply(&mut style, warnings);
    sheet.define_position_try(
        PositionTryRule::new(name, style)
            .in_layer(ctx.layer)
            .in_condition(ctx.condition),
    );
}
