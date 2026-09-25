//! Keyword-valued properties: the generic keyword-table matcher plus
//! the single-keyword enums (`text-decoration`, `overflow`,
//! `scrollbar-gutter`, `position`).

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

pub fn parse_text_decoration(value: &[Token]) -> Option<(bool, bool)> {
    // M1: only single keyword. `underline | line-through | none`.
    if value.len() != 1 {
        return None;
    }
    let name = match &value[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    match name {
        "underline" => Some((true, false)),
        "line-through" => Some((false, true)),
        "none" => Some((false, false)),
        _ => None,
    }
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
