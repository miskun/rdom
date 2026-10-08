//! `line-clamp`, its longhands and the legacy `-webkit-line-clamp` /
//! `-webkit-box-orient` (CSS Overflow 4 §4, Compat Standard).

use super::keyword::parse_keyword;
use super::numeric::components;
use crate::layout::{BlockEllipsis, BoxOrient, Continue};
use crate::parse::token::Token;

/// `max-lines: none | <integer [1,∞]>` (§4.2): `Some(None)` for `none`.
pub fn parse_max_lines(value: &[Token]) -> Option<Option<u32>> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(None),
        [Token::Number(n)] => u32::try_from(*n).ok().filter(|&n| n >= 1).map(Some),
        _ => None,
    }
}

/// `block-ellipsis: no-ellipsis | auto | <string>` (§4.3).
pub fn parse_block_ellipsis(value: &[Token]) -> Option<BlockEllipsis> {
    match value {
        [Token::String(s)] => Some(BlockEllipsis::Str(s.as_str().into())),
        _ => parse_keyword(
            value,
            &[
                ("no-ellipsis", BlockEllipsis::NoEllipsis),
                ("auto", BlockEllipsis::Auto),
            ],
        ),
    }
}

/// `continue: auto | discard | collapse | -webkit-legacy` (§4.4).
pub fn parse_continue(value: &[Token]) -> Option<Continue> {
    parse_keyword(
        value,
        &[
            ("auto", Continue::Auto),
            ("discard", Continue::Discard),
            ("collapse", Continue::Collapse),
            ("-webkit-legacy", Continue::WebkitLegacy),
        ],
    )
}

/// `line-clamp: none | [ <integer [1,∞]> || <'block-ellipsis'> ]
/// -webkit-legacy?` (§4.1): `(max-lines, block-ellipsis, continue)` —
/// `none` is `none`, `no-ellipsis`, `auto`; an integer takes
/// `block-ellipsis: auto` unless one is given, and `continue: collapse`
/// unless `-webkit-legacy` ends the value.
pub fn parse_line_clamp(value: &[Token]) -> Option<(Option<u32>, BlockEllipsis, Continue)> {
    let parts = components(value)?;
    if let [none] = parts.as_slice()
        && matches!(none, [Token::Ident(s)] if s.eq_ignore_ascii_case("none"))
    {
        return Some((None, BlockEllipsis::NoEllipsis, Continue::Auto));
    }
    let (parts, legacy) = match parts.split_last() {
        Some((last, rest)) if parse_continue(last) == Some(Continue::WebkitLegacy) => (rest, true),
        _ => (parts.as_slice(), false),
    };
    let (mut lines, mut ellipsis) = (None, None);
    for part in parts {
        if let Some(Some(n)) = parse_max_lines(part) {
            if lines.replace(n).is_some() {
                return None;
            }
        } else if let Some(e) = parse_block_ellipsis(part) {
            if ellipsis.replace(e).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    let continue_ = if legacy {
        Continue::WebkitLegacy
    } else {
        Continue::Collapse
    };
    Some((
        Some(lines?),
        ellipsis.unwrap_or(BlockEllipsis::Auto),
        continue_,
    ))
}

/// `-webkit-line-clamp: none | <integer [1,∞]>`: as `line-clamp`, but
/// `block-ellipsis` is always `auto` and an integer's `continue`
/// `-webkit-legacy`.
pub fn parse_webkit_line_clamp(value: &[Token]) -> Option<(Option<u32>, BlockEllipsis, Continue)> {
    let lines = parse_max_lines(value)?;
    let continue_ = if lines.is_some() {
        Continue::WebkitLegacy
    } else {
        Continue::Auto
    };
    Some((lines, BlockEllipsis::Auto, continue_))
}

/// `-webkit-box-orient: horizontal | vertical | inline-axis | block-axis`.
pub fn parse_box_orient(value: &[Token]) -> Option<BoxOrient> {
    parse_keyword(
        value,
        &[
            ("horizontal", BoxOrient::Horizontal),
            ("vertical", BoxOrient::Vertical),
            ("inline-axis", BoxOrient::InlineAxis),
            ("block-axis", BoxOrient::BlockAxis),
        ],
    )
}
