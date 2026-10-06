//! The list properties (CSS Lists 3 §3): `list-style-type`,
//! `list-style-position`, `list-style-image`, `list-style`, `marker-side`.

use super::background::image_text;
use super::numeric::components;
use crate::counters::parse_counter_style;
use crate::layout::{ListStyleImage, ListStylePosition, ListStyleType, MarkerSide};
use crate::parse::token::Token;

/// `list-style-type: <counter-style> | <string> | none` (§3.4).
pub fn parse_list_style_type(value: &[Token]) -> Option<ListStyleType> {
    match value {
        [Token::Ident(kw)] if kw.eq_ignore_ascii_case("none") => Some(ListStyleType::None),
        [Token::String(s)] => Some(ListStyleType::String(s.clone())),
        _ => match parse_counter_style(value)? {
            (style, used) if used == value.len() => Some(ListStyleType::Style(style)),
            _ => None,
        },
    }
}

/// `list-style-position: inside | outside` (§3.5).
pub fn parse_list_style_position(value: &[Token]) -> Option<ListStylePosition> {
    let [Token::Ident(kw)] = value else {
        return None;
    };
    match kw.to_ascii_lowercase().as_str() {
        "inside" => Some(ListStylePosition::Inside),
        "outside" => Some(ListStylePosition::Outside),
        _ => None,
    }
}

/// `list-style-image: <image> | none` (§3.3).
pub fn parse_list_style_image(value: &[Token]) -> Option<ListStyleImage> {
    match image_text(value)?.as_str() {
        "none" => Some(ListStyleImage::None),
        text => Some(ListStyleImage::Image(text.to_string())),
    }
}

/// `marker-side: match-self | match-parent` (§3.6).
pub fn parse_marker_side(value: &[Token]) -> Option<MarkerSide> {
    let [Token::Ident(kw)] = value else {
        return None;
    };
    match kw.to_ascii_lowercase().as_str() {
        "match-self" => Some(MarkerSide::MatchSelf),
        "match-parent" => Some(MarkerSide::MatchParent),
        _ => None,
    }
}

/// `list-style: <'list-style-position'> || <'list-style-image'> ||
/// <'list-style-type'>` (§3.6): each component at most once, in any
/// order; `none` sets whichever of image and type the other components
/// left unset — both when neither was set, one `none` then counting for
/// both. An omitted longhand takes its initial value.
pub fn parse_list_style(
    value: &[Token],
) -> Option<(ListStylePosition, ListStyleImage, ListStyleType)> {
    let mut position = None;
    let mut image = None;
    let mut kind = None;
    let mut nones = 0usize;
    for part in components(value)? {
        if let [Token::Ident(kw)] = part
            && kw.eq_ignore_ascii_case("none")
        {
            nones += 1;
            continue;
        }
        if let Some(p) = parse_list_style_position(part) {
            position.replace(p).is_none().then_some(())?;
        } else if let Some(i) = parse_list_style_image(part) {
            image.replace(i).is_none().then_some(())?;
        } else if let Some(t) = parse_list_style_type(part) {
            kind.replace(t).is_none().then_some(())?;
        } else {
            return None;
        }
    }
    // Each `none` fills one of image / type that is still unset.
    let unset = usize::from(image.is_none()) + usize::from(kind.is_none());
    if nones > unset {
        return None;
    }
    if nones > 0 {
        if kind.is_none() {
            kind = Some(ListStyleType::None);
        }
        if image.is_none() {
            image = Some(ListStyleImage::None);
        }
    }
    Some((
        position.unwrap_or_default(),
        image.unwrap_or_default(),
        kind.unwrap_or_default(),
    ))
}
