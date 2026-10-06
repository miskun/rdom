//! The CSS Text properties' value grammars (CSS Text 3 / 4).

use super::numeric::{LengthPercentage, Range, length_percentage, number};
use super::parse_keyword;
use crate::layout::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextAlign, TextAlignLast, TextCase, TextIndent,
    TextJustify, TextTransform, TextWrapMode, TextWrapStyle, WhiteSpaceCollapse, WordBreak,
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

/// `text-indent: <length-percentage> && hanging? && each-line?` (CSS Text
/// 3 §8.1): the keywords anywhere, at most once each, and one length of
/// either sign.
pub fn parse_text_indent(value: &[Token]) -> Option<TextIndent> {
    let (mut hanging, mut each_line) = (false, false);
    let mut rest: Vec<Token> = Vec::with_capacity(value.len());
    for token in value {
        let flag = match token {
            Token::Ident(s) if s.eq_ignore_ascii_case("hanging") => &mut hanging,
            Token::Ident(s) if s.eq_ignore_ascii_case("each-line") => &mut each_line,
            _ => {
                rest.push(token.clone());
                continue;
            }
        };
        if std::mem::replace(flag, true) {
            return None;
        }
    }
    let length = match length_percentage(&rest, Range::Any)? {
        LengthPercentage::Integer(n) => crate::layout::Length::Cells(n),
        LengthPercentage::Cells(v) => crate::layout::Length::Cells(crate::calc::to_cells(v)),
        LengthPercentage::Expr(e) => crate::layout::Length::Calc(Box::new(e)),
    };
    Some(TextIndent {
        length,
        hanging,
        each_line,
    })
}

/// `text-align-all: start | end | left | right | center | justify |
/// match-parent` (CSS Text 3 §6.2).
pub fn parse_text_align_all(value: &[Token]) -> Option<TextAlign> {
    parse_keyword(
        value,
        &[
            ("start", TextAlign::Start),
            ("end", TextAlign::End),
            ("left", TextAlign::Left),
            ("right", TextAlign::Right),
            ("center", TextAlign::Center),
            ("justify", TextAlign::Justify),
            ("match-parent", TextAlign::MatchParent),
        ],
    )
}

/// `text-align-last: auto | <text-align-all's keywords>` (CSS Text 3
/// §6.3).
pub fn parse_text_align_last(value: &[Token]) -> Option<TextAlignLast> {
    if parse_keyword(value, &[("auto", ())]).is_some() {
        return Some(TextAlignLast::Auto);
    }
    parse_text_align_all(value).map(TextAlignLast::of)
}

/// The `text-align` shorthand (CSS Text 3 §6.1): its two longhands —
/// `justify-all` both `justify`, `match-parent` both `match-parent`, any
/// other keyword `text-align-all`'s with `text-align-last: auto`.
pub fn parse_text_align(value: &[Token]) -> Option<(TextAlign, TextAlignLast)> {
    if parse_keyword(value, &[("justify-all", ())]).is_some() {
        return Some((TextAlign::Justify, TextAlignLast::Justify));
    }
    Some(match parse_text_align_all(value)? {
        TextAlign::MatchParent => (TextAlign::MatchParent, TextAlignLast::MatchParent),
        all => (all, TextAlignLast::Auto),
    })
}

/// `text-justify: auto | none | inter-word | inter-character`, with
/// `distribute` a legacy alias of `inter-character` (CSS Text 3 §6.4).
pub fn parse_text_justify(value: &[Token]) -> Option<TextJustify> {
    parse_keyword(
        value,
        &[
            ("auto", TextJustify::Auto),
            ("none", TextJustify::None),
            ("inter-word", TextJustify::InterWord),
            ("inter-character", TextJustify::InterCharacter),
            ("distribute", TextJustify::InterCharacter),
        ],
    )
}

/// `text-wrap-style: auto | balance | stable | pretty |
/// avoid-short-last-line` (CSS Text 4).
pub fn parse_text_wrap_style(value: &[Token]) -> Option<TextWrapStyle> {
    parse_keyword(
        value,
        &[
            ("auto", TextWrapStyle::Auto),
            ("balance", TextWrapStyle::Balance),
            ("stable", TextWrapStyle::Stable),
            ("pretty", TextWrapStyle::Pretty),
            ("avoid-short-last-line", TextWrapStyle::AvoidShortLastLine),
        ],
    )
}

/// `text-wrap: <'text-wrap-mode'> || <'text-wrap-style'>` (CSS Text 4):
/// the two longhands, an omitted one its initial value.
pub fn parse_text_wrap(value: &[Token]) -> Option<(TextWrapMode, TextWrapStyle)> {
    if value.is_empty() || value.len() > 2 {
        return None;
    }
    let (mut mode, mut style) = (None, None);
    for word in value {
        let word = std::slice::from_ref(word);
        if let Some(m) = parse_text_wrap_mode(word) {
            if mode.replace(m).is_some() {
                return None;
            }
        } else if let Some(s) = parse_text_wrap_style(word) {
            if style.replace(s).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    Some((mode.unwrap_or_default(), style.unwrap_or_default()))
}
