//! `scrollbar-gutter` (CSS Overflow 3 §3.3), `scrollbar-width` (CSS
//! Scrollbars 1 §3) and `scrollbar-color` (§2).

use super::{components, parse_color, parse_keyword};
use crate::layout::{ScrollbarColor, ScrollbarGutter, ScrollbarWidth};
use crate::parse::token::Token;

/// `scrollbar-gutter: auto | stable && both-edges?` — `both-edges` only
/// beside `stable`, in either order.
pub fn parse_scrollbar_gutter(value: &[Token]) -> Option<ScrollbarGutter> {
    let is = |t: &[Token], k: &str| matches!(t, [Token::Ident(s)] if s.eq_ignore_ascii_case(k));
    match components(value)?.as_slice() {
        [one] if is(one, "auto") => Some(ScrollbarGutter::Auto),
        [one] if is(one, "stable") => Some(ScrollbarGutter::Stable),
        [a, b]
            if (is(a, "stable") && is(b, "both-edges"))
                || (is(a, "both-edges") && is(b, "stable")) =>
        {
            Some(ScrollbarGutter::StableBothEdges)
        }
        _ => None,
    }
}

/// `scrollbar-width: auto | thin | none`.
pub fn parse_scrollbar_width(value: &[Token]) -> Option<ScrollbarWidth> {
    parse_keyword(
        value,
        &[
            ("auto", ScrollbarWidth::Auto),
            ("thin", ScrollbarWidth::Thin),
            ("none", ScrollbarWidth::None),
        ],
    )
}

/// `scrollbar-color: auto | <color>{2}` — the thumb's color, then the
/// track's.
pub fn parse_scrollbar_color(value: &[Token]) -> Option<ScrollbarColor> {
    if let Some(auto) = parse_keyword(value, &[("auto", ScrollbarColor::Auto)]) {
        return Some(auto);
    }
    match components(value)?.as_slice() {
        [thumb, track] => Some(ScrollbarColor::Colors {
            thumb: parse_color(thumb)?,
            track: parse_color(track)?,
        }),
        _ => None,
    }
}
