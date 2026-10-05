//! The scrolling properties: `overscroll-behavior` (CSS Overscroll
//! Behavior 1 §3).

use super::{components, parse_keyword};
use crate::layout::OverscrollBehavior;
use crate::parse::token::Token;

/// One `overscroll-behavior` keyword: `contain | none | auto`.
pub fn parse_overscroll_behavior(value: &[Token]) -> Option<OverscrollBehavior> {
    parse_keyword(
        value,
        &[
            ("auto", OverscrollBehavior::Auto),
            ("contain", OverscrollBehavior::Contain),
            ("none", OverscrollBehavior::None),
        ],
    )
}

/// The `overscroll-behavior` shorthand: one or two keywords — `x`, then
/// `y` (the second defaults to the first).
pub fn parse_overscroll_behavior_shorthand(
    value: &[Token],
) -> Option<(OverscrollBehavior, OverscrollBehavior)> {
    match components(value)?.as_slice() {
        [both] => parse_overscroll_behavior(both).map(|b| (b, b)),
        [x, y] => Some((parse_overscroll_behavior(x)?, parse_overscroll_behavior(y)?)),
        _ => None,
    }
}
