//! The CSS Text properties' value grammars (CSS Text 3 / 4).

use super::numeric::{LengthPercentage, Range, length_percentage, number};
use super::parse_keyword;
use crate::layout::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextCase, TextTransform, TextWrapMode,
    WhiteSpaceCollapse, WordBreak,
};
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

/// `word-break: normal | break-all | keep-all | break-word` (CSS Text 3
/// §5.2).
pub fn parse_word_break(value: &[Token]) -> Option<WordBreak> {
    parse_keyword(
        value,
        &[
            ("normal", WordBreak::Normal),
            ("break-all", WordBreak::BreakAll),
            ("keep-all", WordBreak::KeepAll),
            ("break-word", WordBreak::BreakWord),
        ],
    )
}

/// `overflow-wrap: normal | break-word | anywhere` (CSS Text 3 §5.5).
pub fn parse_overflow_wrap(value: &[Token]) -> Option<OverflowWrap> {
    parse_keyword(
        value,
        &[
            ("normal", OverflowWrap::Normal),
            ("break-word", OverflowWrap::BreakWord),
            ("anywhere", OverflowWrap::Anywhere),
        ],
    )
}

/// `line-break: auto | loose | normal | strict | anywhere` (CSS Text 3
/// §5.3).
pub fn parse_line_break(value: &[Token]) -> Option<LineBreak> {
    parse_keyword(
        value,
        &[
            ("auto", LineBreak::Auto),
            ("loose", LineBreak::Loose),
            ("normal", LineBreak::Normal),
            ("strict", LineBreak::Strict),
            ("anywhere", LineBreak::Anywhere),
        ],
    )
}

/// `hyphens: none | manual | auto` (CSS Text 3 §6.1).
pub fn parse_hyphens(value: &[Token]) -> Option<Hyphens> {
    parse_keyword(
        value,
        &[
            ("none", Hyphens::None),
            ("manual", Hyphens::Manual),
            ("auto", Hyphens::Auto),
        ],
    )
}

/// `tab-size: <number [0,∞]> | <length [0,∞]>` (CSS Text 3 §4.2): a
/// number of spaces, or a length known at parse time (a percentage or a
/// viewport unit has no basis here and is invalid).
pub fn parse_tab_size(value: &[Token]) -> Option<TabSize> {
    if let Some(n) = number(value, Range::NonNegative) {
        return Some(TabSize::Number(n as f32));
    }
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Cells(c) => Some(TabSize::Length(c as f32)),
        LengthPercentage::Integer(_) | LengthPercentage::Expr(_) => None,
    }
}

/// `text-transform: none | [capitalize | uppercase | lowercase] ||
/// full-width || full-size-kana | math-auto` (CSS Text 4 §2.1).
pub fn parse_text_transform(value: &[Token]) -> Option<TextTransform> {
    match value {
        [] => return None,
        [_] if parse_keyword(value, &[("none", ())]).is_some() => {
            return Some(TextTransform::NONE);
        }
        [_] if parse_keyword(value, &[("math-auto", ())]).is_some() => {
            return Some(TextTransform {
                math_auto: true,
                ..TextTransform::NONE
            });
        }
        _ => {}
    }
    let mut t = TextTransform::NONE;
    let (mut case, mut width, mut kana) = (false, false, false);
    for word in value {
        let word = std::slice::from_ref(word);
        let case_kw = parse_keyword(
            word,
            &[
                ("capitalize", TextCase::Capitalize),
                ("uppercase", TextCase::Uppercase),
                ("lowercase", TextCase::Lowercase),
            ],
        );
        if let Some(c) = case_kw {
            if std::mem::replace(&mut case, true) {
                return None;
            }
            t.case = c;
        } else if parse_keyword(word, &[("full-width", ())]).is_some() {
            if std::mem::replace(&mut width, true) {
                return None;
            }
            t.full_width = true;
        } else if parse_keyword(word, &[("full-size-kana", ())]).is_some() {
            if std::mem::replace(&mut kana, true) {
                return None;
            }
            t.full_size_kana = true;
        } else {
            return None;
        }
    }
    Some(t)
}

/// `text-transform`'s serialization, in the grammar's order.
pub fn serialize_text_transform(t: TextTransform) -> String {
    if t.math_auto {
        return "math-auto".to_string();
    }
    let mut words = Vec::new();
    match t.case {
        TextCase::None => {}
        TextCase::Capitalize => words.push("capitalize"),
        TextCase::Uppercase => words.push("uppercase"),
        TextCase::Lowercase => words.push("lowercase"),
    }
    if t.full_width {
        words.push("full-width");
    }
    if t.full_size_kana {
        words.push("full-size-kana");
    }
    if words.is_empty() {
        "none".to_string()
    } else {
        words.join(" ")
    }
}
