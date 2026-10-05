//! The scrolling properties: `overscroll-behavior` (CSS Overscroll
//! Behavior 1 §3), `scroll-padding` and `scroll-margin` (CSS Scroll Snap
//! 1 §4).

use super::numeric::{LengthPercentage, Range, length_percentage};
use super::spacing::expand_sides;
use super::{components, parse_keyword, parse_padding_value};
use crate::layout::{OverscrollBehavior, ScrollPadding};
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

/// One `scroll-padding` side: `auto | <length-percentage [0,∞]>`.
pub fn parse_scroll_padding(value: &[Token]) -> Option<ScrollPadding> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(ScrollPadding::Auto),
        _ => parse_padding_value(value).map(ScrollPadding::Length),
    }
}

/// The `scroll-padding` shorthand: one to four sides, top, right,
/// bottom, left (CSS Box 3's side order).
pub fn parse_scroll_padding_shorthand(value: &[Token]) -> Option<[ScrollPadding; 4]> {
    let vals = components(value)?
        .into_iter()
        .map(parse_scroll_padding)
        .collect::<Option<Vec<_>>>()?;
    expand_sides(&vals)
}

/// One `scroll-margin` side: a `<length>` of either sign, in whole cells
/// (a percentage is no `<length>`; one relative to the viewport is
/// rejected, as `overflow-clip-margin`'s is — DIVERGENCES).
pub fn parse_scroll_margin(value: &[Token]) -> Option<i16> {
    let cells = match length_percentage(value, Range::Any)? {
        LengthPercentage::Integer(n) => n,
        LengthPercentage::Cells(v) => crate::calc::to_cells(v),
        LengthPercentage::Expr(_) => return None,
    };
    Some(cells.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16)
}

/// The `scroll-margin` shorthand: one to four sides.
pub fn parse_scroll_margin_shorthand(value: &[Token]) -> Option<[i16; 4]> {
    let vals = components(value)?
        .into_iter()
        .map(parse_scroll_margin)
        .collect::<Option<Vec<_>>>()?;
    expand_sides(&vals)
}
