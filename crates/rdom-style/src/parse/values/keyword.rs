//! Keyword-valued properties: the generic keyword-table matcher plus
//! the single-keyword enums (`text-decoration`, `overflow`,
//! `scrollbar-gutter`, `scroll-behavior`, `position`).

use crate::layout::{Overflow, Position};
use crate::parse::token::Token;

pub fn parse_keyword<T: Clone>(value: &[Token], table: &[(&str, T)]) -> Option<T> {
    if value.len() != 1 {
        return None;
    }
    let name = match &value[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    for (k, v) in table {
        if name.eq_ignore_ascii_case(k) {
            return Some(v.clone());
        }
    }
    None
}

/// `text-decoration`: one keyword, `underline | line-through | none`
/// (ASCII case-insensitive, CSS Values 4 §2.1).
pub fn parse_text_decoration(value: &[Token]) -> Option<(bool, bool)> {
    parse_keyword(
        value,
        &[
            ("underline", (true, false)),
            ("line-through", (false, true)),
            ("none", (false, false)),
        ],
    )
}

pub fn parse_overflow(value: &[Token]) -> Option<Overflow> {
    parse_keyword(
        value,
        &[
            ("hidden", Overflow::Hidden),
            ("scroll", Overflow::Scroll),
            ("auto", Overflow::Auto),
            ("visible", Overflow::Visible),
        ],
    )
}

pub fn parse_scrollbar_gutter(value: &[Token]) -> Option<crate::layout::ScrollbarGutter> {
    use crate::layout::ScrollbarGutter;
    parse_keyword(
        value,
        &[
            ("auto", ScrollbarGutter::Auto),
            ("stable", ScrollbarGutter::Stable),
        ],
    )
}

pub fn parse_scroll_behavior(value: &[Token]) -> Option<crate::layout::ScrollBehavior> {
    use crate::layout::ScrollBehavior;
    parse_keyword(
        value,
        &[
            ("auto", ScrollBehavior::Auto),
            ("smooth", ScrollBehavior::Smooth),
        ],
    )
}

pub fn parse_position(value: &[Token]) -> Option<Position> {
    parse_keyword(
        value,
        &[
            ("static", Position::Static),
            ("relative", Position::Relative),
            ("absolute", Position::Absolute),
            ("fixed", Position::Fixed),
            ("sticky", Position::Sticky),
        ],
    )
}
