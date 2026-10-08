//! `cursor` (CSS UI 4 §4.1): `[<url> [<x> <y>]?,]* <cursor-predefined>`.

use super::keyword::parse_keyword;
use super::numeric::{components, split_commas};
use crate::layout::{Cursor, CursorImage, CursorKeyword};
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
