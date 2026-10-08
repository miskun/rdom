//! `@counter-style` (CSS Counter Styles 3 §3).
//!
//! `@counter-style thumbs { system: cyclic; symbols: "👍"; suffix: " " }`
//! defines a counter style in the sheet (`Stylesheet::define_counter_style`),
//! in the cascade layer it sits in. The grammar of each descriptor is
//! `rdom_style::counters::apply_descriptor`'s; an invalid or unknown
//! descriptor is dropped (and reported, `WarningKind::CounterStyleDescriptorDropped`),
//! a rule whose name cannot name a counter style or whose symbols do not
//! suit its system defines nothing (reported once, with
//! `WarningKind::InvalidCounterStyleRule`).

use rdom_style::counters::{
    CounterStyleDefinition, CounterStyleRule, apply_descriptor, check_rule,
};
use rdom_style::parse::Cursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::{LayerId, Stylesheet};

use crate::layer::read_prelude;
use crate::property::read_body;
use crate::{CounterStyleDescriptorReason, CounterStyleRuleReason, Warning, WarningKind};

/// Consume an `@counter-style` rule; the cursor is just past the
/// at-keyword, `at` is the position of `@`, `layer` the cascade layer
/// the rule sits in.
pub(crate) fn consume_counter_style_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    layer: Option<LayerId>,
    at: (u32, u32),
) {
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    let name = prelude.trim().to_string();
    let body = match cursor.peek() {
        Some('{') => {
            cursor.bump();
            read_body(cursor)
        }
        _ => {
            if cursor.peek() == Some(';') {
                cursor.bump();
            }
            None
        }
    };
    let warn = |warnings: &mut Vec<Warning>, kind: WarningKind| {
        warnings.push(Warning {
            kind,
            line: at.0,
            column: at.1,
        });
    };
    let invalid = |reason| WarningKind::InvalidCounterStyleRule {
        name: name.clone(),
        reason,
    };
    let Some(body) = body else {
        warn(warnings, invalid(CounterStyleRuleReason::MissingBlock));
        return;
    };
    let one_ident = matches!(tokenize(&name).as_deref(), Ok([Token::Ident(_)]));
    let (rule, dropped) = match tokenize(&body) {
        Ok(tokens) => descriptors(&tokens),
        Err(_) => {
            warn(warnings, invalid(CounterStyleRuleReason::Unterminated));
            return;
        }
    };
    let checked = if one_ident {
        check_rule(&name, &rule).map_err(CounterStyleRuleReason::Rule)
    } else {
        Err(CounterStyleRuleReason::NotOneIdentifier)
    };
    match checked {
        Ok(()) => {
            for (descriptor, reason) in dropped {
                let kind = WarningKind::CounterStyleDescriptorDropped {
                    name: name.clone(),
                    descriptor,
                    reason,
                };
                warn(warnings, kind);
            }
            sheet.define_counter_style(CounterStyleDefinition::new(&name, rule).in_layer(layer));
        }
        Err(reason) => warn(warnings, invalid(reason)),
    }
}

/// The rule a descriptor block describes, and the declarations it
/// dropped: each one's descriptor name (its text when it has none) and
/// why.
fn descriptors(
    tokens: &[Token],
) -> (
    CounterStyleRule,
    Vec<(String, CounterStyleDescriptorReason)>,
) {
    let mut rule = CounterStyleRule::default();
    let mut dropped = Vec::new();
    for decl in tokens.split(|t| *t == Token::Semicolon) {
        match decl {
            [] => {}
            [Token::Ident(descriptor), Token::Colon, value @ ..] => {
                if let Err(e) = apply_descriptor(&mut rule, descriptor, value) {
                    dropped.push((
                        descriptor.clone(),
                        CounterStyleDescriptorReason::Descriptor(e),
                    ));
                }
            }
            _ => dropped.push((
                rdom_style::parse::values::render_value(decl),
                CounterStyleDescriptorReason::Malformed,
            )),
        }
    }
    (rule, dropped)
}
