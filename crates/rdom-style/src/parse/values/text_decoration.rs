//! CSS Text Decoration 3 / 4 values: `text-decoration` and its longhands,
//! `text-underline-offset`, `text-underline-position`,
//! `text-decoration-skip-ink`.

use super::border::paint_length;
use super::numeric::{Range, components};
use super::{parse_color, parse_keyword};
use crate::TuiColor;
use crate::layout::{
    PaintLength, TextDecorationLine, TextDecorationSkipInk, TextDecorationStyle,
    TextDecorationThickness, TextUnderlineOffset, TextUnderlinePosition,
};
use crate::parse::token::Token;

/// `text-decoration-line: none | [underline || overline || line-through
/// || blink]` (CSS Text Decoration 4 §2.1): each keyword at most once.
pub fn parse_text_decoration_line(value: &[Token]) -> Option<TextDecorationLine> {
    if parse_keyword(value, &[("none", ())]).is_some() {
        return Some(TextDecorationLine::NONE);
    }
    let mut line = TextDecorationLine::NONE;
    for word in value {
        add_line_keyword(&mut line, word)?;
    }
    (!line.is_none()).then_some(line)
}

/// Add one `text-decoration-line` keyword to `line`; `None` when `word`
/// is no such keyword or is already there.
fn add_line_keyword(line: &mut TextDecorationLine, word: &Token) -> Option<()> {
    let Token::Ident(w) = word else {
        return None;
    };
    let flag = match w.to_ascii_lowercase().as_str() {
        "underline" => &mut line.underline,
        "overline" => &mut line.overline,
        "line-through" => &mut line.line_through,
        "blink" => &mut line.blink,
        _ => return None,
    };
    (!std::mem::replace(flag, true)).then_some(())
}

/// `text-decoration-style: solid | double | dotted | dashed | wavy` (§2.3).
pub fn parse_text_decoration_style(value: &[Token]) -> Option<TextDecorationStyle> {
    use TextDecorationStyle as S;
    parse_keyword(
        value,
        &[
            ("solid", S::Solid),
            ("double", S::Double),
            ("dotted", S::Dotted),
            ("dashed", S::Dashed),
            ("wavy", S::Wavy),
        ],
    )
}

/// `text-decoration-thickness: auto | from-font | <length-percentage>`
/// (§2.5): a length of rdom's cells or of the pixel units the decorating
/// properties take (`2px`, `0.1em`, DESIGN "Pixel lengths select").
pub fn parse_text_decoration_thickness(value: &[Token]) -> Option<TextDecorationThickness> {
    if let Some(k) = parse_keyword(
        value,
        &[
            ("auto", TextDecorationThickness::Auto),
            ("from-font", TextDecorationThickness::FromFont),
        ],
    ) {
        return Some(k);
    }
    paint_length(value, true, Range::NonNegative).map(TextDecorationThickness::Length)
}

/// `text-underline-offset: auto | <length-percentage>` (§4.2), of either
/// sign.
pub fn parse_text_underline_offset(value: &[Token]) -> Option<TextUnderlineOffset> {
    if parse_keyword(value, &[("auto", ())]).is_some() {
        return Some(TextUnderlineOffset::Auto);
    }
    paint_length(value, true, Range::Any).map(TextUnderlineOffset::Length)
}

/// `text-underline-position: auto | from-font | [under || [left | right]]`
/// (§4.1).
pub fn parse_text_underline_position(value: &[Token]) -> Option<TextUnderlinePosition> {
    let mut pos = TextUnderlinePosition::AUTO;
    match value {
        [Token::Ident(w)] if w.eq_ignore_ascii_case("auto") => return Some(pos),
        [Token::Ident(w)] if w.eq_ignore_ascii_case("from-font") => {
            pos.from_font = true;
            return Some(pos);
        }
        [] => return None,
        _ => {}
    }
    for word in value {
        let Token::Ident(w) = word else {
            return None;
        };
        match w.to_ascii_lowercase().as_str() {
            "under" if !pos.under => pos.under = true,
            "left" if pos.side.is_none() => pos.side = Some(false),
            "right" if pos.side.is_none() => pos.side = Some(true),
            _ => return None,
        }
    }
    (value.len() <= 2).then_some(pos)
}

/// `text-decoration-skip-ink: auto | none | all` (§3.2).
pub fn parse_text_decoration_skip_ink(value: &[Token]) -> Option<TextDecorationSkipInk> {
    parse_keyword(
        value,
        &[
            ("auto", TextDecorationSkipInk::Auto),
            ("none", TextDecorationSkipInk::None),
            ("all", TextDecorationSkipInk::All),
        ],
    )
}

/// The `text-decoration` shorthand's four longhands (CSS Text Decoration
/// 4 §2.6).
#[derive(Debug, Clone, PartialEq)]
pub struct TextDecorationShorthand {
    pub line: TextDecorationLine,
    pub style: TextDecorationStyle,
    pub color: TuiColor,
    pub thickness: TextDecorationThickness,
}

/// `text-decoration: <'text-decoration-line'> || <'text-decoration-
/// thickness'> || <'text-decoration-style'> || <'text-decoration-color'>`
/// (§2.6): the components in any order, an omitted one its initial value
/// (`none`, `auto`, `solid`, `currentcolor`). The line keywords may sit
/// apart, as long as each comes once.
pub fn parse_text_decoration(value: &[Token]) -> Option<TextDecorationShorthand> {
    let parts = components(value)?;
    if parts.is_empty() {
        return None;
    }
    let mut line: Option<TextDecorationLine> = None;
    let mut none = false;
    let (mut style, mut color, mut thickness) = (None, None, None);
    for part in parts {
        if parse_keyword(part, &[("none", ())]).is_some() && !none && line.is_none() {
            none = true;
            continue;
        }
        if let [word] = part
            && !none
        {
            let mut next = line.unwrap_or(TextDecorationLine::NONE);
            if add_line_keyword(&mut next, word).is_some() {
                line = Some(next);
                continue;
            }
        }
        if style.is_none()
            && let Some(s) = parse_text_decoration_style(part)
        {
            style = Some(s);
        } else if thickness.is_none()
            && let Some(t) = parse_text_decoration_thickness(part)
        {
            thickness = Some(t);
        } else if color.is_none()
            && let Some(c) = parse_color(part)
        {
            color = Some(c);
        } else {
            return None;
        }
    }
    Some(TextDecorationShorthand {
        line: line.unwrap_or(TextDecorationLine::NONE),
        style: style.unwrap_or_default(),
        color: color.unwrap_or(TuiColor::CurrentColor),
        thickness: thickness.unwrap_or_default(),
    })
}

/// A decoration length's serialization: rdom's cells bare, pixels in
/// `px`, an expression as written.
pub fn serialize_decoration_length(length: &PaintLength) -> String {
    match length {
        PaintLength::Cells(c) => format!("{c}"),
        PaintLength::Px(px) => format!("{px}px"),
        PaintLength::Calc(expr) => crate::property_dispatch::serialize_math(expr),
    }
}

/// `text-decoration-thickness`'s serialization.
pub fn serialize_text_decoration_thickness(t: &TextDecorationThickness) -> String {
    match t {
        TextDecorationThickness::Auto => "auto".to_string(),
        TextDecorationThickness::FromFont => "from-font".to_string(),
        TextDecorationThickness::Length(l) => serialize_decoration_length(l),
    }
}
