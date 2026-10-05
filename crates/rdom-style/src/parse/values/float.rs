//! `float` and `clear` (CSS 2.1 §9.5.1 / §9.5.2, CSS Logical 1 §2.3).

use super::parse_keyword;
use crate::layout::{Clear, Float};
use crate::parse::token::Token;

/// `float: none | left | right | inline-start | inline-end`.
pub fn parse_float(value: &[Token]) -> Option<Float> {
    parse_keyword(
        value,
        &[
            ("none", Float::None),
            ("left", Float::Left),
            ("right", Float::Right),
            ("inline-start", Float::InlineStart),
            ("inline-end", Float::InlineEnd),
        ],
    )
}

/// `clear: none | left | right | both | inline-start | inline-end`.
pub fn parse_clear(value: &[Token]) -> Option<Clear> {
    parse_keyword(
        value,
        &[
            ("none", Clear::None),
            ("left", Clear::Left),
            ("right", Clear::Right),
            ("both", Clear::Both),
            ("inline-start", Clear::InlineStart),
            ("inline-end", Clear::InlineEnd),
        ],
    )
}
