//! `@counter-style` (CSS Counter Styles 3 §3).
//!
//! `@counter-style thumbs { system: cyclic; symbols: "👍"; suffix: " " }`
//! defines a counter style in the sheet (`Stylesheet::define_counter_style`),
//! in the cascade layer it sits in. The grammar of each descriptor is
//! `rdom_style::counters::apply_descriptor`'s; an invalid or unknown
//! descriptor is dropped (and reported), a rule whose name cannot name a
//! counter style or whose symbols do not suit its system defines nothing
//! (reported once, with `WarningKind::InvalidCounterStyleRule`).

use rdom_style::counters::{
    CounterStyleDefinition, CounterStyleRule, apply_descriptor, check_rule,
};
use rdom_style::parse::Cursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::{LayerId, Stylesheet};

use crate::layer::read_prelude;
use crate::property::read_body;
use crate::{Warning, WarningKind};

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
    let warn = |warnings: &mut Vec<Warning>, reason: String| {
        warnings.push(Warning {
            kind: WarningKind::InvalidCounterStyleRule {
                name: name.clone(),
                reason,
            },
            line: at.0,
            column: at.1,
        });
    };
    let Some(body) = body else {
        warn(
            warnings,
            "an `@counter-style` rule needs a block".to_string(),
        );
        return;
    };
    let one_ident = matches!(tokenize(&name).as_deref(), Ok([Token::Ident(_)]));
    let (rule, dropped) = match tokenize(&body) {
        Ok(tokens) => descriptors(&tokens),
        Err(_) => {
            warn(warnings, "unterminated string or comment".to_string());
            return;
        }
    };
    let checked = if one_ident {
        check_rule(&name, &rule)
    } else {
        Err(format!("`{name}` is not one identifier"))
    };
    match checked {
        Ok(()) => {
            for reason in dropped {
                warn(warnings, reason);
            }
            sheet.define_counter_style(CounterStyleDefinition::new(&name, rule).in_layer(layer));
        }
        Err(reason) => warn(warnings, reason),
    }
}

/// The rule a descriptor block describes, and the reasons of the
/// declarations it dropped.
fn descriptors(tokens: &[Token]) -> (CounterStyleRule, Vec<String>) {
    let mut rule = CounterStyleRule::default();
    let mut dropped = Vec::new();
    for decl in tokens.split(|t| *t == Token::Semicolon) {
        match decl {
            [] => {}
            [Token::Ident(descriptor), Token::Colon, value @ ..] => {
                if let Err(reason) = apply_descriptor(&mut rule, descriptor, value) {
                    dropped.push(reason);
                }
            }
            _ => dropped.push("a declaration that is not `descriptor: value`".to_string()),
        }
    }
    (rule, dropped)
}
