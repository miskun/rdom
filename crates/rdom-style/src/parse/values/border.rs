//! Border values (CSS Backgrounds 3 §4): the `border` and
//! `border-<side>` shorthands (`<line-width> || <line-style> ||
//! <color>`), line styles and widths, the current-border read used by
//! the per-side style longhands, and the [`PaintLength`] leaf that
//! border widths (and radii, shadows) share.

use super::color::parse_color;
use super::keyword::parse_keyword;
use super::numeric::{LengthPercentage, Range, components, length_percentage};
use crate::layout::{Border, BorderStyle, BorderWidth, CornerStyle, PaintLength};
use crate::parse::token::Token;
use crate::{TuiColor, TuiStyle, Value};

/// Read the current border from `style`, defaulting to all-sides-off
/// when nothing is set. Used by the `border-top-style` /
/// `border-right-style` / … longhands so consecutive declarations
/// combine instead of overwriting.
pub fn current_border(style: &TuiStyle) -> Border {
    match style.border {
        Some(Value::Specified(b)) => b,
        _ => Border::none(),
    }
}

/// The CSS `<line-style>` keywords (§4.2) and rdom's `half-block`, plus
/// the rdom synonyms `single` and `rounded` for `solid` (`rounded`
/// rounds the corners only in the `border` shorthand).
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
    .or_else(|| paint_length(value, false).map(BorderWidth::Length))
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

/// A non-negative [`PaintLength`]: a pixel-unit dimension, or rdom's
/// `<length>` (cells, `ch`, `lh`, viewport units, math functions) —
/// with a percentage too when `percent` (radii).
pub(crate) fn paint_length(value: &[Token], percent: bool) -> Option<PaintLength> {
    if let [Token::Dimension { value: n, unit, .. }] = value
        && let Some((_, px)) = PX_PER_UNIT
            .iter()
            .find(|(u, _)| u.eq_ignore_ascii_case(unit))
    {
        return (*n >= 0.0).then(|| PaintLength::Px((n * px) as f32));
    }
    match length_percentage(value, Range::NonNegative)? {
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

/// `border` (§4.4): the same grammar on all four sides, which it sets
/// together; or one of rdom's one-side keywords `top` / `right` /
/// `bottom` / `left` alone (that side solid, the others none). Returns
/// the per-side styles (with the corner style) and the shared width and
/// color.
pub fn parse_border(value: &[Token]) -> Option<(Border, BorderWidth, TuiColor)> {
    let one_side = parse_keyword(
        value,
        &[
            ("top", Border::top()),
            ("bottom", Border::bottom()),
            ("left", Border::left()),
            ("right", Border::right()),
        ],
    );
    if let Some(b) = one_side {
        return Some((b, BorderWidth::Medium, TuiColor::CurrentColor));
    }
    let s = parse_border_side_shorthand(value)?;
    let mut border = Border::ring(s.style);
    if s.rounded {
        border.corner_style = CornerStyle::Rounded;
    }
    Some((border, s.width, s.color))
}
