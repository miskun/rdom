//! The CSS Multi-column Layout 1 values (`column-count`, `column-width`,
//! the `columns` and `column-rule` shorthands, `column-span`,
//! `column-fill`) and the CSS Fragmentation 3 values (`break-*`, the
//! legacy `page-break-*`, `orphans`, `widows`, `box-decoration-break`).

use super::border::{parse_border_side, parse_line_width};
use super::color::parse_color;
use super::keyword::parse_keyword;
use super::numeric::{LengthPercentage, Range, cells_u16, components, integer, length_percentage};
use crate::TuiColor;
use crate::layout::{
    BorderStyle, BorderWidth, BoxDecorationBreak, BreakBetween, BreakInside, ColumnCount,
    ColumnFill, ColumnSpan, ColumnWidth,
};
use crate::parse::token::Token;

fn is_auto(value: &[Token]) -> bool {
    matches!(value, [Token::Ident(s)] if s.eq_ignore_ascii_case("auto"))
}

/// An `<integer [1,∞]>`, clamped to `u32`.
fn positive_integer(value: &[Token]) -> Option<u32> {
    let n = integer(value)?;
    (n >= 1).then(|| n.min(i64::from(u32::MAX)) as u32)
}

/// `column-count` (§3.2): `auto | <integer [1,∞]>`.
pub fn parse_column_count(value: &[Token]) -> Option<ColumnCount> {
    if is_auto(value) {
        return Some(ColumnCount::Auto);
    }
    positive_integer(value).map(ColumnCount::Count)
}

/// `column-width` (§3.1): `auto | <length [0,∞]>` — cells (a bare number
/// included, rdom's cell), `ch`, a viewport unit or a math function of
/// them; no percentage, and no pixel or font-relative length (geometry).
pub fn parse_column_width(value: &[Token]) -> Option<ColumnWidth> {
    if is_auto(value) {
        return Some(ColumnWidth::Auto);
    }
    match length_percentage(value, Range::NonNegative)? {
        LengthPercentage::Integer(n) => u16::try_from(n).ok().map(ColumnWidth::Cells),
        LengthPercentage::Cells(v) => Some(ColumnWidth::Cells(cells_u16(v))),
        LengthPercentage::Expr(e) if !e.contains_percent() => {
            Some(ColumnWidth::Calc(std::sync::Arc::new(e)))
        }
        LengthPercentage::Expr(_) => None,
    }
}

/// A bare number other than `0`: in `columns`, CSS's `<integer>` — never
/// rdom's unitless cell length, so `columns: 3 4` stays invalid as in a
/// browser (a bare `0` is a `<length>` in CSS).
fn is_bare_nonzero_number(value: &[Token]) -> bool {
    match value {
        [Token::Number(n)] => *n != 0,
        [Token::Float(_)] => true,
        _ => false,
    }
}

/// The `columns` shorthand (§3.3): `<'column-width'> || <'column-count'>`,
/// each omitted one `auto`. A bare number is the count (CSS's
/// `<integer>`; rdom's unitless cell length is `column-width`'s only on
/// its own), a length with a unit (`20ch`) the width; `auto` fills
/// whichever is left. `(width, count)`.
pub fn parse_columns(value: &[Token]) -> Option<(ColumnWidth, ColumnCount)> {
    let parts = components(value)?;
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let mut width = None;
    let mut count = None;
    let mut autos = 0;
    for part in &parts {
        if is_auto(part) {
            autos += 1;
        } else if count.is_none()
            && let Some(n) = positive_integer(part)
        {
            count = Some(ColumnCount::Count(n));
        } else if width.is_none()
            && !is_bare_nonzero_number(part)
            && let Some(w) = parse_column_width(part)
            && w != ColumnWidth::Auto
        {
            width = Some(w);
        } else {
            return None;
        }
    }
    // Each `auto` stands for one of the two; more `auto`s than unset
    // components would repeat one.
    let unset = usize::from(width.is_none()) + usize::from(count.is_none());
    if autos > unset {
        return None;
    }
    Some((
        width.unwrap_or(ColumnWidth::Auto),
        count.unwrap_or(ColumnCount::Auto),
    ))
}

/// `column-rule-style` (§4.2): a `<line-style>`.
pub fn parse_column_rule_style(value: &[Token]) -> Option<BorderStyle> {
    parse_border_side(value)
}

/// `column-rule-width` (§4.3): a `<line-width>`.
pub fn parse_column_rule_width(value: &[Token]) -> Option<BorderWidth> {
    parse_line_width(value)
}

/// `column-rule-color` (§4.1): a `<color>`.
pub fn parse_column_rule_color(value: &[Token]) -> Option<TuiColor> {
    parse_color(value)
}

/// The `column-rule` shorthand (§4.4): `<'column-rule-width'> ||
/// <'column-rule-style'> || <'column-rule-color'>`, each omitted one at
/// its initial value. `(width, style, color)`.
pub fn parse_column_rule(value: &[Token]) -> Option<(BorderWidth, BorderStyle, TuiColor)> {
    let mut width = None;
    let mut style = None;
    let mut color = None;
    for part in components(value)? {
        if style.is_none()
            && let Some(s) = parse_column_rule_style(part)
        {
            style = Some(s);
        } else if width.is_none()
            && let Some(w) = parse_column_rule_width(part)
        {
            width = Some(w);
        } else if color.is_none()
            && let Some(c) = parse_column_rule_color(part)
        {
            color = Some(c);
        } else {
            return None;
        }
    }
    if width.is_none() && style.is_none() && color.is_none() {
        return None;
    }
    Some((
        width.unwrap_or(BorderWidth::Medium),
        style.unwrap_or(BorderStyle::None),
        color.unwrap_or(TuiColor::CurrentColor),
    ))
}

/// `column-span` (§6.1): `none | all`.
pub fn parse_column_span(value: &[Token]) -> Option<ColumnSpan> {
    parse_keyword(value, ColumnSpan::KEYWORDS)
}

/// `column-fill` (§7.1): `auto | balance | balance-all`.
pub fn parse_column_fill(value: &[Token]) -> Option<ColumnFill> {
    parse_keyword(value, ColumnFill::KEYWORDS)
}

/// `break-before` / `break-after` (CSS Fragmentation 3 §3.1, 4 §3.1).
pub fn parse_break_between(value: &[Token]) -> Option<BreakBetween> {
    parse_keyword(value, BreakBetween::KEYWORDS)
}

/// `break-inside` (§3.2).
pub fn parse_break_inside(value: &[Token]) -> Option<BreakInside> {
    parse_keyword(value, BreakInside::KEYWORDS)
}

/// The legacy `page-break-before` / `page-break-after` (CSS Fragmentation
/// 3 §3.4): `auto | always | avoid | left | right`, as `break-before` /
/// `break-after` with `always` the `page` value.
pub fn parse_page_break_between(value: &[Token]) -> Option<BreakBetween> {
    parse_keyword(
        value,
        &[
            ("auto", BreakBetween::Auto),
            ("always", BreakBetween::Page),
            ("avoid", BreakBetween::Avoid),
            ("left", BreakBetween::Left),
            ("right", BreakBetween::Right),
        ],
    )
}

/// The `page-break-before` / `-after` spelling of a `break-before` /
/// `-after` value; `None` for one the legacy property cannot write.
pub fn page_break_between_keyword(v: BreakBetween) -> Option<&'static str> {
    Some(match v {
        BreakBetween::Auto => "auto",
        BreakBetween::Page => "always",
        BreakBetween::Avoid => "avoid",
        BreakBetween::Left => "left",
        BreakBetween::Right => "right",
        _ => return None,
    })
}

/// The legacy `page-break-inside` (§3.4): `auto | avoid`.
pub fn parse_page_break_inside(value: &[Token]) -> Option<BreakInside> {
    parse_keyword(
        value,
        &[("auto", BreakInside::Auto), ("avoid", BreakInside::Avoid)],
    )
}

/// `orphans` / `widows` (§3.3): `<integer [1,∞]>`.
pub fn parse_orphans_widows(value: &[Token]) -> Option<u32> {
    positive_integer(value)
}

/// `box-decoration-break` (§5.4): `slice | clone`.
pub fn parse_box_decoration_break(value: &[Token]) -> Option<BoxDecorationBreak> {
    parse_keyword(value, BoxDecorationBreak::KEYWORDS)
}
