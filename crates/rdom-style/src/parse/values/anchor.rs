//! The CSS Anchor Positioning 1 properties' values: `anchor-name`,
//! `anchor-scope`, `position-anchor`, `position-area`,
//! `position-try-fallbacks`, `position-try-order`, the `position-try`
//! shorthand and `position-visibility`. (`anchor()` / `anchor-size()` are
//! math leaves, `calc.rs`.)

use std::sync::Arc;

use super::keyword::parse_keyword;
use super::numeric::split_commas;
use crate::layout::{
    AnchorName, AnchorScope, AreaKeyword, PositionAnchor, PositionArea, PositionTryOrder,
    PositionVisibility, TryFallback, TryTactic,
};
use crate::parse::token::Token;

/// A `<dashed-ident>`: `--` and at least one more character.
fn dashed(t: &Token) -> Option<Arc<str>> {
    match t {
        Token::Ident(s) if s.starts_with("--") && s.len() > 2 => Some(s.as_str().into()),
        _ => None,
    }
}

fn is(value: &[Token], keyword: &str) -> bool {
    matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case(keyword))
}

/// A `<dashed-ident>#` list.
fn dashed_list(value: &[Token]) -> Option<Vec<Arc<str>>> {
    split_commas(value)?
        .into_iter()
        .map(|part| match part {
            [t] => dashed(t),
            _ => None,
        })
        .collect()
}

/// `anchor-name` (§2.1): `none | <dashed-ident>#`.
pub fn parse_anchor_name(value: &[Token]) -> Option<AnchorName> {
    if is(value, "none") {
        return Some(AnchorName::none());
    }
    Some(AnchorName::new(dashed_list(value)?))
}

/// `anchor-scope` (§2.2): `none | all | <dashed-ident>#`.
pub fn parse_anchor_scope(value: &[Token]) -> Option<AnchorScope> {
    if is(value, "none") {
        return Some(AnchorScope::None);
    }
    if is(value, "all") {
        return Some(AnchorScope::All);
    }
    Some(AnchorScope::Names(dashed_list(value)?.into()))
}

/// `position-anchor` (§2.3): `auto | none | <anchor-name>`.
pub fn parse_position_anchor(value: &[Token]) -> Option<PositionAnchor> {
    if is(value, "auto") {
        return Some(PositionAnchor::Auto);
    }
    if is(value, "none") {
        return Some(PositionAnchor::None);
    }
    match value {
        [t] => dashed(t).map(PositionAnchor::Name),
        _ => None,
    }
}

/// One `<position-area>` (§3.1): one or two keywords of one of its
/// vocabularies.
fn position_area(value: &[Token]) -> Option<PositionArea> {
    let keyword = |t: &Token| parse_keyword(std::slice::from_ref(t), AreaKeyword::KEYWORDS);
    match value {
        [a] => PositionArea::new(keyword(a)?, None),
        [a, b] => PositionArea::new(keyword(a)?, Some(keyword(b)?)),
        _ => None,
    }
}

/// `position-area` (§3.1): `none | <position-area>`; `None` inside is
/// `none`.
pub fn parse_position_area(value: &[Token]) -> Option<Option<PositionArea>> {
    if is(value, "none") {
        return Some(None);
    }
    position_area(value).map(Some)
}

/// `position-try-fallbacks` (§4.1): `none | [ [<dashed-ident> ||
/// <try-tactic>] | <'position-area'> ]#`, a `<try-tactic>` being
/// `flip-block || flip-inline || flip-start` (and Anchor Positioning 2's
/// `flip-x`, `flip-y`), each at most once. Empty is `none`.
pub fn parse_position_try_fallbacks(value: &[Token]) -> Option<Vec<TryFallback>> {
    if is(value, "none") {
        return Some(Vec::new());
    }
    split_commas(value)?
        .into_iter()
        .map(|entry| {
            if let Some(area) = position_area(entry) {
                return Some(TryFallback::area(area));
            }
            let mut name = None;
            let mut tactics = Vec::new();
            for t in entry {
                if let Some(n) = dashed(t) {
                    if name.is_some() {
                        return None;
                    }
                    name = Some(n);
                } else {
                    let tactic = parse_keyword(std::slice::from_ref(t), TryTactic::KEYWORDS)?;
                    if tactics.contains(&tactic) {
                        return None;
                    }
                    tactics.push(tactic);
                }
            }
            (name.is_some() || !tactics.is_empty()).then(|| TryFallback::rule(name, tactics))
        })
        .collect()
}

/// `position-try-order` (§4.2): `normal | <try-size>`.
pub fn parse_position_try_order(value: &[Token]) -> Option<PositionTryOrder> {
    parse_keyword(value, PositionTryOrder::KEYWORDS)
}

/// The `position-try` shorthand (§4.3): `<'position-try-order'>?
/// <'position-try-fallbacks'>`.
pub fn parse_position_try(value: &[Token]) -> Option<(PositionTryOrder, Vec<TryFallback>)> {
    if let Some((first, rest)) = value.split_first()
        && let Some(order) = parse_position_try_order(std::slice::from_ref(first))
        && !rest.is_empty()
    {
        return Some((order, parse_position_try_fallbacks(rest)?));
    }
    Some((
        PositionTryOrder::Normal,
        parse_position_try_fallbacks(value)?,
    ))
}

/// `position-visibility` (§5): `always | [ anchors-valid ||
/// anchors-visible || no-overflow ]`.
pub fn parse_position_visibility(value: &[Token]) -> Option<PositionVisibility> {
    if is(value, "always") {
        return Some(PositionVisibility::ALWAYS);
    }
    let mut v = PositionVisibility::ALWAYS;
    if value.is_empty() {
        return None;
    }
    for t in value {
        let Token::Ident(k) = t else {
            return None;
        };
        let flag = match k.to_ascii_lowercase().as_str() {
            "anchors-valid" => &mut v.anchors_valid,
            "anchors-visible" => &mut v.anchors_visible,
            "no-overflow" => &mut v.no_overflow,
            _ => return None,
        };
        if *flag {
            return None;
        }
        *flag = true;
    }
    Some(v)
}
