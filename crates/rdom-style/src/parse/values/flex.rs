//! The flex container keywords: `flex-direction` (CSS Flexbox §5.1),
//! `flex-wrap` (§5.2) and their `flex-flow` shorthand (§5.3).

use super::keyword::parse_keyword;
use crate::layout::{Direction, FlexWrap};
use crate::parse::token::Token;

const DIRECTIONS: &[(&str, (Direction, bool))] = &[
    ("row", (Direction::Row, false)),
    ("row-reverse", (Direction::Row, true)),
    ("column", (Direction::Column, false)),
    ("column-reverse", (Direction::Column, true)),
];

const WRAPS: &[(&str, FlexWrap)] = &[
    ("nowrap", FlexWrap::NoWrap),
    ("wrap", FlexWrap::Wrap),
    ("wrap-reverse", FlexWrap::WrapReverse),
];

/// `flex-direction`: the axis and whether its start and end swap
/// (`row-reverse` / `column-reverse`).
pub fn parse_flex_direction(value: &[Token]) -> Option<(Direction, bool)> {
    parse_keyword(value, DIRECTIONS)
}

/// `flex-wrap: nowrap | wrap | wrap-reverse`.
pub fn parse_flex_wrap(value: &[Token]) -> Option<FlexWrap> {
    parse_keyword(value, WRAPS)
}

/// `flex-flow: <'flex-direction'> || <'flex-wrap'>` — one or both, in
/// either order; an omitted one is its initial value (`row`, `nowrap`).
pub fn parse_flex_flow(value: &[Token]) -> Option<((Direction, bool), FlexWrap)> {
    if value.is_empty() || value.len() > 2 {
        return None;
    }
    let (mut direction, mut wrap) = (None, None);
    for token in value {
        let one = std::slice::from_ref(token);
        if direction.is_none()
            && let Some(d) = parse_flex_direction(one)
        {
            direction = Some(d);
        } else if wrap.is_none()
            && let Some(w) = parse_flex_wrap(one)
        {
            wrap = Some(w);
        } else {
            return None;
        }
    }
    Some((
        direction.unwrap_or((Direction::Row, false)),
        wrap.unwrap_or(FlexWrap::NoWrap),
    ))
}

/// `flex-direction`'s keyword.
pub fn serialize_flex_direction(direction: Direction, reverse: bool) -> &'static str {
    match (direction, reverse) {
        (Direction::Row, false) => "row",
        (Direction::Row, true) => "row-reverse",
        (Direction::Column, false) => "column",
        (Direction::Column, true) => "column-reverse",
    }
}

/// `flex-wrap`'s keyword.
pub fn serialize_flex_wrap(wrap: FlexWrap) -> &'static str {
    match wrap {
        FlexWrap::NoWrap => "nowrap",
        FlexWrap::Wrap => "wrap",
        FlexWrap::WrapReverse => "wrap-reverse",
    }
}

/// `flex-flow` in its shortest form (CSSOM §6.7.2): a component at its
/// initial value is left out, unless both are.
pub fn serialize_flex_flow(direction: Direction, reverse: bool, wrap: FlexWrap) -> String {
    let d = serialize_flex_direction(direction, reverse);
    match (d, wrap) {
        (d, FlexWrap::NoWrap) => d.to_string(),
        ("row", w) => serialize_flex_wrap(w).to_string(),
        (d, w) => format!("{d} {}", serialize_flex_wrap(w)),
    }
}
