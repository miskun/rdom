//! The CSS Text properties (CSS Text 3 / 4): `white-space` and its
//! longhands `white-space-collapse` / `text-wrap-mode`, `word-break`,
//! `overflow-wrap` (and its legacy name `word-wrap`), `line-break`,
//! `hyphens`, `tab-size`, `text-transform`, `text-indent`, `text-align`
//! (and its longhands `text-align-all` / `text-align-last`),
//! `text-justify` — their `set` and `serialize` arms.

use super::value_serializers::{serialize_length, specified};
use crate::layout::{TextWrapMode, WhiteSpace, WhiteSpaceCollapse};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_hyphens, parse_line_break, parse_overflow_wrap, parse_tab_size, parse_text_align,
    parse_text_align_all, parse_text_align_last, parse_text_indent, parse_text_justify,
    parse_text_transform, parse_text_wrap_mode, parse_white_space, parse_white_space_collapse,
    parse_word_break, serialize_text_transform,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let text = &mut style.text;
    Some(match name {
        "white-space" => parse_white_space(value).map(|(c, m)| {
            text.white_space_collapse = Some(Value::Specified(c));
            text.text_wrap_mode = Some(Value::Specified(m));
        }),
        "white-space-collapse" => parse_white_space_collapse(value).map(|c| {
            text.white_space_collapse = Some(Value::Specified(c));
        }),
        "text-wrap-mode" => parse_text_wrap_mode(value).map(|m| {
            text.text_wrap_mode = Some(Value::Specified(m));
        }),
        "word-break" => parse_word_break(value).map(|w| {
            text.word_break = Some(Value::Specified(w));
        }),
        "overflow-wrap" | "word-wrap" => parse_overflow_wrap(value).map(|w| {
            text.overflow_wrap = Some(Value::Specified(w));
        }),
        "line-break" => parse_line_break(value).map(|l| {
            text.line_break = Some(Value::Specified(l));
        }),
        "hyphens" => parse_hyphens(value).map(|h| {
            text.hyphens = Some(Value::Specified(h));
        }),
        "tab-size" => parse_tab_size(value).map(|t| {
            text.tab_size = Some(Value::Specified(t));
        }),
        "text-transform" => parse_text_transform(value).map(|t| {
            text.text_transform = Some(Value::Specified(t));
        }),
        "text-indent" => parse_text_indent(value).map(|t| {
            text.text_indent = Some(Value::Specified(t));
        }),
        "text-align" => parse_text_align(value).map(|(all, last)| {
            text.text_align_all = Some(Value::Specified(all));
            text.text_align_last = Some(Value::Specified(last));
        }),
        "text-align-all" => parse_text_align_all(value).map(|a| {
            text.text_align_all = Some(Value::Specified(a));
        }),
        "text-align-last" => parse_text_align_last(value).map(|l| {
            text.text_align_last = Some(Value::Specified(l));
        }),
        "text-justify" => parse_text_justify(value).map(|j| {
            text.text_justify = Some(Value::Specified(j));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let text = &style.text;
    let collapse = text.white_space_collapse.as_ref().and_then(specified);
    let mode = text.text_wrap_mode.as_ref().and_then(specified);
    Some(match name {
        "white-space-collapse" => collapse.map(|c| c.keyword().to_string()),
        "text-wrap-mode" => mode.map(|m| m.keyword().to_string()),
        "word-break" => keyword(&text.word_break, |w| w.keyword()),
        "overflow-wrap" | "word-wrap" => keyword(&text.overflow_wrap, |w| w.keyword()),
        "line-break" => keyword(&text.line_break, |l| l.keyword()),
        "hyphens" => keyword(&text.hyphens, |h| h.keyword()),
        "text-align-all" => keyword(&text.text_align_all, |a| a.keyword()),
        "text-align-last" => keyword(&text.text_align_last, |l| l.keyword()),
        "text-justify" => keyword(&text.text_justify, |j| j.keyword()),
        "text-align" => {
            use crate::layout::{TextAlign, TextAlignLast};
            let all = text.text_align_all.as_ref().and_then(specified);
            let last = text.text_align_last.as_ref().and_then(specified);
            match (all, last) {
                (Some(TextAlign::Justify), Some(TextAlignLast::Justify)) => {
                    Some("justify-all".to_string())
                }
                (Some(TextAlign::MatchParent), Some(TextAlignLast::MatchParent)) => {
                    Some("match-parent".to_string())
                }
                (Some(a), Some(TextAlignLast::Auto)) if *a != TextAlign::MatchParent => {
                    Some(a.keyword().to_string())
                }
                _ => None,
            }
        }
        "text-indent" => text.text_indent.as_ref().and_then(specified).map(|t| {
            let mut out = serialize_length(&t.length);
            if t.hanging {
                out.push_str(" hanging");
            }
            if t.each_line {
                out.push_str(" each-line");
            }
            out
        }),
        "text-transform" => text
            .text_transform
            .as_ref()
            .and_then(specified)
            .map(|t| serialize_text_transform(*t)),
        "tab-size" => text.tab_size.as_ref().and_then(specified).map(|t| match t {
            crate::layout::TabSize::Number(n) => format!("{n}"),
            crate::layout::TabSize::Length(c) => format!("{c}ch"),
        }),
        "white-space" => match (collapse, mode) {
            (Some(&c), Some(&m)) => Some(white_space_text(c, m)),
            _ => None,
        },
        _ => return None,
    })
}

/// The shortest `white-space` that sets `collapse` and `mode` (CSSOM
/// §6.7.2's shortest-form rule): the keyword the pair spells, else the
/// longhand values that are not initial.
fn white_space_text(collapse: WhiteSpaceCollapse, mode: TextWrapMode) -> String {
    if let Some(w) = WhiteSpace::from_longhands(collapse, mode) {
        return w.keyword().to_string();
    }
    match mode {
        TextWrapMode::Wrap => collapse.keyword().to_string(),
        TextWrapMode::Nowrap => format!("{} {}", collapse.keyword(), mode.keyword()),
    }
}

/// A keyword field's serialization: its value's spelling, when specified.
fn keyword<T>(field: &Option<Value<T>>, spell: impl Fn(&T) -> &'static str) -> Option<String> {
    field
        .as_ref()
        .and_then(specified)
        .map(|v| spell(v).to_string())
}
