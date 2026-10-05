//! The CSS Text properties' value grammars (CSS Text 3 / 4).

use super::parse_keyword;
use crate::layout::{TextWrapMode, WhiteSpaceCollapse};
use crate::parse::token::Token;

/// `white-space-collapse: collapse | preserve | preserve-breaks |
/// preserve-spaces | break-spaces` (CSS Text 4 §4.1).
pub fn parse_white_space_collapse(value: &[Token]) -> Option<WhiteSpaceCollapse> {
    use WhiteSpaceCollapse as C;
    parse_keyword(
        value,
        &[
            ("collapse", C::Collapse),
            ("preserve", C::Preserve),
            ("preserve-breaks", C::PreserveBreaks),
            ("preserve-spaces", C::PreserveSpaces),
            ("break-spaces", C::BreakSpaces),
        ],
    )
}

/// `text-wrap-mode: wrap | nowrap` (CSS Text 4 §6.1).
pub fn parse_text_wrap_mode(value: &[Token]) -> Option<TextWrapMode> {
    parse_keyword(
        value,
        &[
            ("wrap", TextWrapMode::Wrap),
            ("nowrap", TextWrapMode::Nowrap),
        ],
    )
}

/// `white-space: normal | pre | pre-wrap | pre-line |
/// <'white-space-collapse'> || <'text-wrap-mode'>` (CSS Text 4 §3; the
/// `white-space-trim` component is not supported): the two longhands, an
/// omitted one its initial value.
pub fn parse_white_space(value: &[Token]) -> Option<(WhiteSpaceCollapse, TextWrapMode)> {
    use crate::layout::WhiteSpace;
    if let Some(w) = parse_keyword(
        value,
        &[
            ("normal", WhiteSpace::Normal),
            ("pre", WhiteSpace::Pre),
            ("pre-wrap", WhiteSpace::PreWrap),
            ("pre-line", WhiteSpace::PreLine),
        ],
    ) {
        return Some(w.longhands());
    }
    if value.is_empty() || value.len() > 2 {
        return None;
    }
    let (mut collapse, mut mode) = (None, None);
    for word in value {
        let word = std::slice::from_ref(word);
        if let Some(c) = parse_white_space_collapse(word) {
            if collapse.replace(c).is_some() {
                return None;
            }
        } else if let Some(m) = parse_text_wrap_mode(word) {
            if mode.replace(m).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    Some((collapse.unwrap_or_default(), mode.unwrap_or_default()))
}
