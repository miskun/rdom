//! `cursor` (CSS UI 4 §4.1): `[<url> [<x> <y>]?,]* <cursor-predefined>` —
//! and the caret's values (§6.2): `caret-shape`, `caret-animation`, the
//! `caret` shorthand.

use super::color::parse_color;
use super::keyword::parse_keyword;
use super::numeric::{components, split_commas};
use crate::layout::{
    AccentColor, Appearance, CaretAnimation, CaretColor, CaretShape, Cursor, CursorImage,
    CursorKeyword,
};
use crate::parse::token::Token;

/// `cursor`: image fallbacks, each a URL with an optional hotspot of two
/// numbers, then the keyword (which must end the list).
pub fn parse_cursor(value: &[Token]) -> Option<Cursor> {
    let items = split_commas(value)?;
    let (last, images) = items.split_last()?;
    let keyword = parse_keyword(last, CursorKeyword::KEYWORDS)?;
    let images = images
        .iter()
        .map(|item| parse_image(item))
        .collect::<Option<Vec<_>>>()?;
    Some(Cursor {
        images: images.into(),
        keyword,
    })
}

/// `<url> [<x> <y>]?`.
fn parse_image(item: &[Token]) -> Option<CursorImage> {
    let parts = components(item)?;
    let (url, rest) = parts.split_first()?;
    let url = match url {
        [Token::Url(u)] => u.clone(),
        [Token::Function(f), Token::String(u), Token::RParen] if f.eq_ignore_ascii_case("url") => {
            u.clone()
        }
        _ => return None,
    };
    let number = |t: &[Token]| -> Option<f32> {
        let (sign, t) = match t {
            [Token::Delim('-'), rest @ ..] => (-1.0, rest),
            _ => (1.0, t),
        };
        match t {
            [Token::Number(n)] => Some(sign * *n as f32),
            [Token::Float(f)] => Some(sign * *f as f32),
            _ => None,
        }
    };
    let hotspot = match rest {
        [] => None,
        [x, y] => Some((number(x)?, number(y)?)),
        _ => return None,
    };
    Some(CursorImage { url, hotspot })
}

/// `caret-color` (§6.1): `auto | <color>` — and rdom's `transparent`
/// keyword, a color that suppresses the caret.
pub fn parse_caret_color(value: &[Token]) -> Option<CaretColor> {
    parse_keyword(
        value,
        &[
            ("auto", CaretColor::Auto),
            ("transparent", CaretColor::Transparent),
        ],
    )
    .or_else(|| parse_color(value).map(CaretColor::Color))
}

/// `caret-shape` (§6.2.2).
pub fn parse_caret_shape(value: &[Token]) -> Option<CaretShape> {
    parse_keyword(value, CaretShape::KEYWORDS)
}

/// `caret-animation` (§6.2.1).
pub fn parse_caret_animation(value: &[Token]) -> Option<CaretAnimation> {
    parse_keyword(value, CaretAnimation::KEYWORDS)
}

/// The `caret` shorthand (§6.2.3): `<'caret-color'> || <'caret-animation'>
/// || <'caret-shape'>`, each omitted one at its initial value; an `auto`
/// goes to the first of them not yet given.
pub fn parse_caret(value: &[Token]) -> Option<(CaretColor, CaretAnimation, CaretShape)> {
    let (mut color, mut animation, mut shape) = (None, None, None);
    for part in components(value)? {
        if color.is_none()
            && let Some(c) = parse_caret_color(part)
        {
            color = Some(c);
        } else if animation.is_none()
            && let Some(a) = parse_caret_animation(part)
        {
            animation = Some(a);
        } else if shape.is_none()
            && let Some(s) = parse_caret_shape(part)
        {
            shape = Some(s);
        } else {
            return None;
        }
    }
    Some((
        color.unwrap_or_default(),
        animation.unwrap_or_default(),
        shape.unwrap_or_default(),
    ))
}

/// `accent-color` (§6.3): `auto | <color>`.
pub fn parse_accent_color(value: &[Token]) -> Option<AccentColor> {
    parse_keyword(value, &[("auto", AccentColor::Auto)])
        .or_else(|| parse_color(value).map(AccentColor::Color))
}

/// `appearance` (§7.1).
pub fn parse_appearance(value: &[Token]) -> Option<Appearance> {
    parse_keyword(value, Appearance::KEYWORDS)
}
