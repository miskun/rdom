//! The outline values (CSS UI 4 §5): `outline-style`, `outline-color`,
//! `outline-offset` and the `outline` shorthand; `outline-width` is a
//! `<line-width>` ([`parse_line_width`]).

use super::border::{paint_length, parse_line_width};
use super::color::parse_color;
use super::keyword::parse_keyword;
use super::numeric::{Range, components};
use crate::layout::{BorderWidth, OutlineColor, OutlineStyle, PaintLength};
use crate::parse::token::Token;

/// `outline-style` (§5.2): `auto | <outline-line-style>` — every
/// `<line-style>` but `hidden`.
pub fn parse_outline_style(value: &[Token]) -> Option<OutlineStyle> {
    parse_keyword(value, OutlineStyle::KEYWORDS)
}

/// `outline-color` (§5.3): `auto | <color>`.
pub fn parse_outline_color(value: &[Token]) -> Option<OutlineColor> {
    parse_keyword(value, &[("auto", OutlineColor::Auto)])
        .or_else(|| parse_color(value).map(OutlineColor::Color))
}

/// `outline-offset` (§5.4): a `<length>` of either sign — rdom's cells,
/// or a pixel length, which selects a one-cell offset its way
/// ([`PaintLength::offset_cells`]).
pub fn parse_outline_offset(value: &[Token]) -> Option<PaintLength> {
    paint_length(value, false, Range::Any)
}

/// The `outline` shorthand (§5.1): `<'outline-color'> || <'outline-style'>
/// || <'outline-width'>`, each omitted one at its initial value. A lone
/// `auto` is the style (the one browsers take), a second the color.
pub fn parse_outline(value: &[Token]) -> Option<(OutlineColor, OutlineStyle, BorderWidth)> {
    let mut color = None;
    let mut style = None;
    let mut width = None;
    for part in components(value)? {
        if style.is_none()
            && let Some(s) = parse_outline_style(part)
        {
            style = Some(s);
        } else if width.is_none()
            && let Some(w) = parse_line_width(part)
        {
            width = Some(w);
        } else if color.is_none()
            && let Some(c) = parse_outline_color(part)
        {
            color = Some(c);
        } else {
            return None;
        }
    }
    Some((
        color.unwrap_or_default(),
        style.unwrap_or_default(),
        width.unwrap_or_default(),
    ))
}
