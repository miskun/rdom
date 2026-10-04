//! Border values (CSS Backgrounds 3 §4–§5): the `border` and
//! `border-<side>` shorthands (`<line-width> || <line-style> ||
//! <color>`), line styles and widths, corner radii, and the
//! [`PaintLength`] leaf that border widths, radii and shadows share.

use super::color::parse_color;
use super::keyword::parse_keyword;
use super::numeric::{LengthPercentage, Range, components, length_percentage};
use crate::TuiColor;
use crate::layout::{Border, BorderRadius, BorderStyle, BorderWidth, Corners, PaintLength, Sides};
use crate::parse::token::Token;

/// The CSS `<line-style>` keywords (§4.2) and rdom's `half-block`, plus
/// the rdom synonyms `single` and `rounded` for `solid` (`rounded` also
/// rounds the corners in the `border` shorthand).
const LINE_STYLES: &[(&str, BorderStyle)] = &[
    ("none", BorderStyle::None),
    ("hidden", BorderStyle::Hidden),
    ("solid", BorderStyle::Solid),
    ("single", BorderStyle::Solid),
    ("rounded", BorderStyle::Solid),
    ("half-block", BorderStyle::HalfBlock),
    ("double", BorderStyle::Double),
    ("dashed", BorderStyle::Dashed),
    ("dotted", BorderStyle::Dotted),
    ("ridge", BorderStyle::Ridge),
    ("outset", BorderStyle::Outset),
    ("groove", BorderStyle::Groove),
    ("inset", BorderStyle::Inset),
];

/// One `<line-style>` (the `border-*-style` longhands). Unknown values
/// → `None` so the caller emits a warning.
pub fn parse_border_side(value: &[Token]) -> Option<BorderStyle> {
    parse_keyword(value, LINE_STYLES)
}

/// One to four values of `one`, expanded clockwise from the top (CSS
/// Backgrounds 3 §4.1–§4.3: `border-color`, `border-style`,
/// `border-width`).
pub fn parse_sides<T: Clone>(value: &[Token], one: fn(&[Token]) -> Option<T>) -> Option<Sides<T>> {
    let values = components(value)?
        .into_iter()
        .map(one)
        .collect::<Option<Vec<T>>>()?;
    Sides::from_values(&values)
}

/// One `<line-width>` (§4.3): `thin | medium | thick | <length
/// [0,∞]>`. A length is rdom's cells or, since a width only picks a
/// glyph weight, a pixel or `em` length ([`PaintLength`]).
pub fn parse_line_width(value: &[Token]) -> Option<BorderWidth> {
    parse_keyword(
        value,
        &[
            ("thin", BorderWidth::Thin),
            ("medium", BorderWidth::Medium),
            ("thick", BorderWidth::Thick),
        ],
    )
    .or_else(|| paint_length(value, false, Range::NonNegative).map(BorderWidth::Length))
}

/// CSS pixels per unit for the units a [`PaintLength`] takes beside
/// rdom's cells (CSS Values 4 §6.2: 1in = 96px = 2.54cm; `em` / `rem`
/// at the initial font size, 16px).
const PX_PER_UNIT: &[(&str, f64)] = &[
    ("px", 1.0),
    ("cm", 96.0 / 2.54),
    ("mm", 96.0 / 25.4),
    ("q", 96.0 / 101.6),
    ("in", 96.0),
    ("pt", 96.0 / 72.0),
    ("pc", 16.0),
    ("em", 16.0),
    ("rem", 16.0),
];

/// A [`PaintLength`]: a pixel-unit dimension, or rdom's `<length>`
/// (cells, `ch`, `lh`, viewport units, math functions) — with a
/// percentage too when `percent` (radii), and of any sign when `range`
/// is [`Range::Any`] (shadow offsets).
pub(crate) fn paint_length(value: &[Token], percent: bool, range: Range) -> Option<PaintLength> {
    let (negative, rest) = match value {
        [Token::Delim('-'), rest @ ..] => (true, rest),
        _ => (false, value),
    };
    if let [Token::Dimension { value: n, unit, .. }] = rest
        && let Some((_, px)) = PX_PER_UNIT
            .iter()
            .find(|(u, _)| u.eq_ignore_ascii_case(unit))
    {
        let sign = if negative { -1.0 } else { 1.0 };
        let ok = range == Range::Any || !negative;
        return ok.then(|| PaintLength::Px((sign * n * px) as f32));
    }
    match length_percentage(value, range)? {
        LengthPercentage::Integer(n) => Some(PaintLength::Cells(n as f32)),
        LengthPercentage::Cells(c) => Some(PaintLength::Cells(c as f32)),
        LengthPercentage::Expr(e) if percent || !e.contains_percent() => {
            Some(PaintLength::Calc(Box::new(e)))
        }
        LengthPercentage::Expr(_) => None,
    }
}

/// A parsed `border` / `border-<side>` shorthand: `<line-width> ||
/// <line-style> || <color>`, each omitted component at its initial
/// value (`medium`, `none`, `currentcolor`; §4.4).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderShorthand {
    pub width: BorderWidth,
    pub style: BorderStyle,
    pub color: TuiColor,
    /// rdom's `rounded` keyword: a solid line with rounded corners.
    pub rounded: bool,
}

/// `border-top` / `-right` / `-bottom` / `-left` (§4.4).
pub fn parse_border_side_shorthand(value: &[Token]) -> Option<BorderShorthand> {
    let parts = components(value)?;
    let mut out = BorderShorthand {
        width: BorderWidth::Medium,
        style: BorderStyle::None,
        color: TuiColor::CurrentColor,
        rounded: false,
    };
    let (mut width, mut style, mut color) = (false, false, false);
    for part in parts {
        if !style && let Some(s) = parse_border_side(part) {
            out.style = s;
            out.rounded = matches!(part, [Token::Ident(k)] if k.eq_ignore_ascii_case("rounded"));
            style = true;
        } else if !width && let Some(w) = parse_line_width(part) {
            out.width = w;
            width = true;
        } else if !color && let Some(c) = parse_color(part) {
            out.color = c;
            color = true;
        } else {
            return None;
        }
    }
    Some(out)
}

/// A parsed `border` shorthand: the per-side styles and the width and
/// color every side takes; `rounded` when it was rdom's `rounded`
/// (a solid ring that also sets `border-radius: 1`).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderRing {
    pub styles: Border,
    pub width: BorderWidth,
    pub color: TuiColor,
    pub rounded: bool,
}

/// `border` (§4.4): the same grammar on all four sides, which it sets
/// together; or one of rdom's one-side keywords `top` / `right` /
/// `bottom` / `left` alone (that side solid, the others none).
pub fn parse_border(value: &[Token]) -> Option<BorderRing> {
    let one_side = parse_keyword(
        value,
        &[
            ("top", Border::top()),
            ("bottom", Border::bottom()),
            ("left", Border::left()),
            ("right", Border::right()),
        ],
    );
    if let Some(styles) = one_side {
        return Some(BorderRing {
            styles,
            width: BorderWidth::Medium,
            color: TuiColor::CurrentColor,
            rounded: false,
        });
    }
    let s = parse_border_side_shorthand(value)?;
    Some(BorderRing {
        styles: Border::ring(s.style),
        width: s.width,
        color: s.color,
        rounded: s.rounded,
    })
}

/// One corner's `border-*-radius` (§5.1): `<length-percentage
/// [0,∞]>{1,2}` — the horizontal radius, then the vertical one (the
/// horizontal again when omitted).
pub fn parse_corner_radius(value: &[Token]) -> Option<BorderRadius> {
    match components(value)?.as_slice() {
        [h] => paint_length(h, true, Range::NonNegative).map(BorderRadius::circle),
        [h, v] => Some(BorderRadius {
            horizontal: paint_length(h, true, Range::NonNegative)?,
            vertical: paint_length(v, true, Range::NonNegative)?,
        }),
        _ => None,
    }
}

/// `border-radius` (§5.2): `<length-percentage [0,∞]>{1,4} [ /
/// <length-percentage [0,∞]>{1,4} ]?` — the horizontal radii of the
/// four corners clockwise from the top-left, then the vertical ones
/// (the horizontal ones when omitted).
pub fn parse_border_radius(value: &[Token]) -> Option<Corners<BorderRadius>> {
    // The top-level `/` (one inside `calc()` divides).
    let mut depth = 0usize;
    let slash = value.iter().position(|t| {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            _ => {}
        }
        depth == 0 && *t == Token::Delim('/')
    });
    let (h, v) = match slash {
        Some(i) => (&value[..i], Some(&value[i + 1..])),
        None => (value, None),
    };
    let radii = |part: &[Token]| -> Option<Corners<PaintLength>> {
        let values = components(part)?
            .into_iter()
            .map(|c| paint_length(c, true, Range::NonNegative))
            .collect::<Option<Vec<_>>>()?;
        Corners::from_values(&values)
    };
    let horizontal = radii(h)?;
    let vertical = match v {
        Some(v) => radii(v)?,
        None => horizontal.clone(),
    };
    Some(
        horizontal
            .zip(vertical)
            .map(|(horizontal, vertical)| BorderRadius {
                horizontal,
                vertical,
            }),
    )
}

/// `border-spacing` (CSS 2.1 §17.6.1): `<length [0,∞]> <length
/// [0,∞]>?` — horizontal, then vertical (the horizontal when omitted);
/// rdom's cell lengths, no percentages.
pub fn parse_border_spacing(value: &[Token]) -> Option<crate::layout::BorderSpacing> {
    use crate::layout::GapValue;
    let one = |c: &[Token]| match length_percentage(c, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(GapValue::Cells),
        LengthPercentage::Cells(v) => Some(GapValue::Cells(super::numeric::cells_u16(v))),
        LengthPercentage::Expr(e) if !e.contains_percent() => Some(GapValue::Calc(Box::new(e))),
        LengthPercentage::Expr(_) => None,
    };
    let (horizontal, vertical) = match components(value)?.as_slice() {
        [h] => {
            let h = one(h)?;
            (h.clone(), h)
        }
        [h, v] => (one(h)?, one(v)?),
        _ => return None,
    };
    Some(crate::layout::BorderSpacing {
        horizontal,
        vertical,
    })
}
