//! CSS Fonts 4 values: `font-weight`, `font-style`, `font-size`,
//! `font-family`, `font-stretch`, `font-variant`, and the `font`
//! shorthand.

use super::border::paint_length;
use super::numeric::{Range, components, number, parse_angle};
use super::parse_keyword;
use crate::layout::{
    FontFamily, FontSize, FontSizeKeyword, FontStretch, FontStretchKeyword, FontStyle, FontVariant,
    FontWeight, LineHeight, SystemFont,
};
use crate::parse::token::Token;

/// `font-weight: <font-weight-absolute> | bolder | lighter` (CSS Fonts 4
/// §2.2): `normal`, `bold`, or a number in `[1, 1000]` (a math function's
/// clamped there, §10.9).
pub fn parse_font_weight(value: &[Token]) -> Option<FontWeight> {
    if let Some(k) = parse_keyword(
        value,
        &[
            ("normal", FontWeight::Normal),
            ("bold", FontWeight::Bold),
            ("bolder", FontWeight::Bolder),
            ("lighter", FontWeight::Lighter),
        ],
    ) {
        return Some(k);
    }
    let literal = matches!(value, [Token::Number(_) | Token::Float(_)]);
    let n = number(value, Range::NonNegative)?;
    if literal && !(1.0..=1000.0).contains(&n) {
        return None;
    }
    Some(FontWeight::Number(n.clamp(1.0, 1000.0) as f32))
}

/// `font-style: normal | italic | oblique <angle [-90deg,90deg]>?` (CSS
/// Fonts 4 §2.4).
pub fn parse_font_style(value: &[Token]) -> Option<FontStyle> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("normal") => Some(FontStyle::Normal),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("italic") => Some(FontStyle::Italic),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("oblique") => Some(FontStyle::Oblique(None)),
        [Token::Ident(s), angle @ ..] if s.eq_ignore_ascii_case("oblique") => {
            let degrees = parse_angle(angle)?;
            (-90.0..=90.0)
                .contains(&degrees)
                .then_some(FontStyle::Oblique(Some(degrees as f32)))
        }
        _ => None,
    }
}

/// `font-size: <absolute-size> | <relative-size> | <length-percentage
/// [0,∞]> | math` (CSS Fonts 4 §2.5); a length in the pixel units the
/// decorating properties take (`16px`, `1.2em`) or rdom's cells.
pub fn parse_font_size(value: &[Token]) -> Option<FontSize> {
    if let [Token::Ident(s)] = value {
        if s.eq_ignore_ascii_case("medium") {
            return Some(FontSize::Medium);
        }
        if s.eq_ignore_ascii_case("math") {
            return Some(FontSize::Math);
        }
        return FontSizeKeyword::ALL
            .iter()
            .find(|(_, w)| s.eq_ignore_ascii_case(w))
            .map(|(k, _)| FontSize::Keyword(*k));
    }
    paint_length(value, true, Range::NonNegative).map(FontSize::Length)
}

/// `font-family: [<family-name> | <generic-family>]#` (CSS Fonts 4
/// §2.1): each a string or a sequence of identifiers (not a CSS-wide
/// keyword, nor `default`).
pub fn parse_font_family(value: &[Token]) -> Option<FontFamily> {
    let mut names = Vec::new();
    for item in super::numeric::split_commas(value)? {
        names.push(family_name(item)?);
    }
    (!names.is_empty()).then(|| FontFamily::Names(names.into()))
}

/// One family name as it serializes: a string quoted, identifiers joined
/// by a space.
fn family_name(item: &[Token]) -> Option<String> {
    match item {
        [Token::String(s)] => Some(format!(
            "\"{}\"",
            s.replace('\\', "\\\\").replace('"', "\\\"")
        )),
        words if !words.is_empty() => {
            let mut out: Vec<&str> = Vec::with_capacity(words.len());
            for w in words {
                let Token::Ident(w) = w else {
                    return None;
                };
                out.push(w);
            }
            let reserved = [
                "inherit",
                "initial",
                "unset",
                "default",
                "revert",
                "revert-layer",
            ];
            if out.len() == 1 && reserved.iter().any(|r| out[0].eq_ignore_ascii_case(r)) {
                return None;
            }
            Some(out.join(" "))
        }
        _ => None,
    }
}

/// `font-stretch: normal | <percentage [0,∞]> | ultra-condensed | … |
/// ultra-expanded` (CSS Fonts 4 §2.3).
pub fn parse_font_stretch(value: &[Token]) -> Option<FontStretch> {
    if let [Token::Ident(s)] = value {
        if s.eq_ignore_ascii_case("normal") {
            return Some(FontStretch::Normal);
        }
        return FontStretchKeyword::from_keyword(s).map(FontStretch::Keyword);
    }
    match value {
        [Token::Percentage(p)] if *p >= 0.0 => Some(FontStretch::Percent(*p as f32)),
        _ => None,
    }
}

/// `font-variant` in CSS 2.1's form: `normal | small-caps`.
pub fn parse_font_variant(value: &[Token]) -> Option<FontVariant> {
    parse_keyword(
        value,
        &[
            ("normal", FontVariant::Normal),
            ("small-caps", FontVariant::SmallCaps),
        ],
    )
}

/// The `font` shorthand's longhands (CSS Fonts 4 §3.7), `line-height`
/// with them.
#[derive(Debug, Clone, PartialEq)]
pub struct FontShorthand {
    pub style: FontStyle,
    pub variant: FontVariant,
    pub weight: FontWeight,
    pub stretch: FontStretch,
    pub size: FontSize,
    pub line_height: LineHeight,
    pub family: FontFamily,
}

/// `font: [ [ <'font-style'> || <font-variant-css2> || <'font-weight'> ||
/// <font-width-css3> ]? <'font-size'> [ / <'line-height'> ]?
/// <'font-family'># ] | <system-family-name>` (CSS Fonts 4 §3.7): an
/// omitted longhand is its initial value (`normal`, the line height
/// included). A system font sets the family to it and the rest to their
/// initial values.
pub fn parse_font(value: &[Token]) -> Option<FontShorthand> {
    let initial = FontShorthand {
        style: FontStyle::Normal,
        variant: FontVariant::Normal,
        weight: FontWeight::Normal,
        stretch: FontStretch::Normal,
        size: FontSize::Medium,
        line_height: LineHeight::Normal,
        family: FontFamily::Initial,
    };
    if let [Token::Ident(s)] = value
        && let Some((system, _)) = SystemFont::ALL
            .iter()
            .find(|(_, w)| s.eq_ignore_ascii_case(w))
    {
        return Some(FontShorthand {
            family: FontFamily::System(*system),
            ..initial
        });
    }
    let parts = components(value)?;
    let mut font = initial;
    let (mut style, mut variant, mut weight, mut stretch) = (false, false, false, false);
    let mut i = 0;
    // The optional prefix, any order, each at most once; `normal` sets
    // whichever is still unset.
    while let Some(part) = parts.get(i) {
        if parse_keyword(part, &[("normal", ())]).is_some() {
            i += 1;
            continue;
        }
        if !style && let Some(s) = parse_font_style_prefix(&parts, &mut i) {
            font.style = s;
            style = true;
            continue;
        }
        if !variant && let Some(v) = parse_font_variant(part) {
            font.variant = v;
            variant = true;
        } else if !weight
            && let Some(w) = parse_font_weight(part)
                .filter(|w| !matches!(w, FontWeight::Bolder | FontWeight::Lighter))
        {
            font.weight = w;
            weight = true;
        } else if !stretch
            && let Some(s) =
                parse_font_stretch(part).filter(|s| !matches!(s, FontStretch::Percent(_)))
        {
            font.stretch = s;
            stretch = true;
        } else {
            break;
        }
        i += 1;
    }
    // The size, then `/ <line-height>`, then the family list.
    font.size = parse_font_size(parts.get(i)?)?;
    i += 1;
    if parts
        .get(i)
        .is_some_and(|p| matches!(p, [Token::Delim('/')]))
    {
        font.line_height = super::parse_line_height(parts.get(i + 1)?)?;
        i += 2;
    }
    // The family list is the rest of `value`: the components partition it.
    let rest: usize = parts.get(i..)?.iter().map(|p| p.len()).sum();
    if rest == 0 {
        return None;
    }
    font.family = parse_font_family(&value[value.len() - rest..])?;
    Some(font)
}

/// `font-style` in the `font` shorthand's prefix: `oblique` takes the
/// angle after it when there is one.
fn parse_font_style_prefix(parts: &[&[Token]], i: &mut usize) -> Option<FontStyle> {
    let part = parts.get(*i)?;
    if parse_keyword(part, &[("oblique", ())]).is_some()
        && let Some(angle) = parts.get(*i + 1)
        && let Some(degrees) = parse_angle(angle)
    {
        if !(-90.0..=90.0).contains(&degrees) {
            return None;
        }
        *i += 2;
        return Some(FontStyle::Oblique(Some(degrees as f32)));
    }
    let style = parse_keyword(
        part,
        &[
            ("italic", FontStyle::Italic),
            ("oblique", FontStyle::Oblique(None)),
        ],
    )?;
    *i += 1;
    Some(style)
}

/// `font-weight`'s serialization.
pub fn serialize_font_weight(w: FontWeight) -> String {
    match w {
        FontWeight::Normal => "normal".to_string(),
        FontWeight::Bold => "bold".to_string(),
        FontWeight::Bolder => "bolder".to_string(),
        FontWeight::Lighter => "lighter".to_string(),
        FontWeight::Number(n) => format!("{n}"),
    }
}

/// `font-style`'s serialization.
pub fn serialize_font_style(s: FontStyle) -> String {
    match s {
        FontStyle::Normal => "normal".to_string(),
        FontStyle::Italic => "italic".to_string(),
        FontStyle::Oblique(None) => "oblique".to_string(),
        FontStyle::Oblique(Some(deg)) => format!("oblique {deg}deg"),
    }
}

/// `font-size`'s serialization.
pub fn serialize_font_size(s: &FontSize) -> String {
    match s {
        FontSize::Medium => "medium".to_string(),
        FontSize::Math => "math".to_string(),
        FontSize::Keyword(k) => k.keyword().to_string(),
        FontSize::Length(l) => super::serialize_decoration_length(l),
    }
}

/// `font-family`'s serialization; `None` for the initial value, which
/// has no spelling of its own.
pub fn serialize_font_family(f: &FontFamily) -> Option<String> {
    match f {
        FontFamily::Initial => None,
        FontFamily::Names(names) => Some(names.join(", ")),
        FontFamily::System(s) => Some(s.keyword().to_string()),
    }
}

/// `font-stretch`'s serialization.
pub fn serialize_font_stretch(s: FontStretch) -> String {
    match s {
        FontStretch::Normal => "normal".to_string(),
        FontStretch::Keyword(k) => k.keyword().to_string(),
        FontStretch::Percent(p) => format!("{p}%"),
    }
}

/// `font-variant`'s serialization.
pub fn serialize_font_variant(v: FontVariant) -> &'static str {
    match v {
        FontVariant::Normal => "normal",
        FontVariant::SmallCaps => "small-caps",
    }
}
