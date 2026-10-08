//! `@keyframes` (CSS Animations 1 §3).
//!
//! `@keyframes slide { from { left: 0 } 50%, 75% { left: 4 } to { left: 8 } }`
//! defines the keyframes `slide` in the sheet
//! (`Stylesheet::define_keyframes`), in the cascade layer it sits in. The
//! name is a `<custom-ident>` (not `none`, a CSS-wide keyword or
//! `default`) or a `<string>`; an invalid one drops the rule
//! (`WarningKind::InvalidAtRulePrelude`). Each keyframe block is a
//! `<keyframe-selector>#` — `from`, `to`, percentages in [0%, 100%], a
//! timeline range name and a percentage (Scroll-driven Animations 1 §4.4) —
//! and a declaration list: a block with an invalid selector is ignored
//! (`WarningKind::InvalidKeyframeSelector`), and an `!important`
//! declaration in a block is ignored
//! (`WarningKind::ImportantInKeyframe`). Blocks with the same selector
//! are all kept; they cascade when the rule is resolved
//! (`KeyframesRule::resolve`).

use rdom_style::keyframes::{Keyframe, KeyframeSelector, KeyframesRule, TimelineRangeName};
use rdom_style::parse::SourceCursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::{RuleContext, Stylesheet, TuiStyle};

use crate::declarations::DeclarationRun;
use crate::layer::read_prelude;
use crate::property::read_body;
use crate::top_level::{skip_at_rule_rest, skip_ws_and_comments};
use crate::{Warning, WarningKind};

/// Consume an `@keyframes` rule; the cursor is just past the
/// at-keyword, `at` is the position of `@`, `ctx` the cascade layer
/// and the conditional group rule it sits in.
pub(crate) fn consume_keyframes_rule(
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
        Ok([t]) => rdom_style::parse::values::keyframes_name(t),
        _ => None,
    };
    let has_block = cursor.peek() == Some('{');
    let Some(name) = name.filter(|_| has_block) else {
        warnings.push(Warning {
            kind: WarningKind::InvalidAtRulePrelude {
                name: "keyframes".to_string(),
                prelude: prelude.trim().to_string(),
            },
            line: at.0,
            column: at.1,
        });
        skip_at_rule_rest(cursor, warnings, false);
        return;
    };
    cursor.bump(); // '{'
    let mut rule = KeyframesRule::new(name)
        .in_layer(ctx.layer)
        .in_condition(ctx.condition);
    while let Some(keyframe) = next_keyframe(cursor, warnings) {
        if let Some(keyframe) = keyframe {
            rule = rule.with(keyframe);
        }
    }
    sheet.define_keyframes(rule);
}

/// The next keyframe block of the rule's body: `Some(Some(_))` for a
/// block, `Some(None)` for one dropped (warned), `None` at the body's
/// end (its `}` consumed, or EOF).
fn next_keyframe(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
) -> Option<Option<Keyframe>> {
    if !skip_ws_and_comments(cursor, warnings) {
        return None;
    }
    match cursor.peek()? {
        '}' => {
            cursor.bump();
            return None;
        }
        '@' => {
            // No at-rule belongs in a keyframes body (§3).
            let (line, column) = (cursor.line(), cursor.col());
            cursor.bump();
            let (name, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
            cursor.advance(used);
            warnings.push(Warning {
                kind: WarningKind::UnsupportedAtRule(name),
                line,
                column,
            });
            skip_at_rule_rest(cursor, warnings, true);
            return Some(None);
        }
        _ => {}
    }
    let (line, column) = (cursor.line(), cursor.col());
    let prelude = read_prelude(cursor, warnings)?;
    if cursor.peek() != Some('{') {
        // `;` or EOF: not a keyframe block.
        cursor.bump();
        warnings.push(Warning {
            kind: WarningKind::InvalidKeyframeSelector(prelude.trim().to_string()),
            line,
            column,
        });
        return Some(None);
    }
    cursor.bump(); // '{'
    let (body_line, body_col) = (cursor.line(), cursor.col());
    let body = read_body(cursor).unwrap_or_default();
    let Some(selectors) = selectors(prelude.trim()) else {
        warnings.push(Warning {
            kind: WarningKind::InvalidKeyframeSelector(prelude.trim().to_string()),
            line,
            column,
        });
        return Some(None);
    };
    let mut run = DeclarationRun::default();
    run.push(&body, body_line, body_col, warnings);
    run.drop_important(warnings);
    let mut style = TuiStyle::new();
    run.apply(&mut style, warnings);
    Some(Some(Keyframe::new(selectors, style)))
}

/// A `<keyframe-selector>#`: `from` (0%), `to` (100%) or a percentage in
/// [0%, 100%] each; `None` when any is invalid.
fn selectors(prelude: &str) -> Option<Vec<KeyframeSelector>> {
    let tokens = tokenize(prelude).ok()?;
    let out = tokens
        .split(|t| *t == Token::Comma)
        .map(|seg| match seg {
            [Token::Ident(s)] if s.eq_ignore_ascii_case("from") => KeyframeSelector::at(0.0),
            [Token::Ident(s)] if s.eq_ignore_ascii_case("to") => KeyframeSelector::at(1.0),
            [Token::Percentage(p)] => KeyframeSelector::at((*p / 100.0) as f32),
            // Scroll-driven Animations 1 §4.4: `<timeline-range-name>
            // <percentage>`.
            [Token::Ident(name), Token::Percentage(p)] => KeyframeSelector::in_range(
                TimelineRangeName::from_keyword(name)?,
                (*p / 100.0) as f32,
            ),
            [Token::Ident(name), Token::Delim('-'), Token::Percentage(p)] => {
                KeyframeSelector::in_range(
                    TimelineRangeName::from_keyword(name)?,
                    (-*p / 100.0) as f32,
                )
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!out.is_empty()).then_some(out)
}
