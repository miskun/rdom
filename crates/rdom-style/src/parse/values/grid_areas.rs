//! Named grid areas (CSS Grid Layout 2 §7.3): `grid-template-areas` —
//! `none | <string>+` — and its serialization, each string's tokens one
//! space apart and each null cell token a single `.`.

use crate::layout::GridTemplateAreas;
use crate::parse::token::Token;

/// Parse `grid-template-areas` (§7.3): `none`, or one string per row
/// that together mark out rectangular named areas
/// ([`GridTemplateAreas::new`] refuses anything else).
pub fn parse_grid_template_areas(value: &[Token]) -> Option<GridTemplateAreas> {
    if let [Token::Ident(s)] = value
        && s.eq_ignore_ascii_case("none")
    {
        return Some(GridTemplateAreas::NONE);
    }
    let strings = value
        .iter()
        .map(|t| match t {
            Token::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    GridTemplateAreas::new(strings)
}

/// The text of a `grid-template-areas` value: `none`, or its rows as
/// strings.
pub fn serialize_grid_template_areas(areas: &GridTemplateAreas) -> String {
    if areas.is_none() {
        return "none".to_string();
    }
    (0..areas.row_count())
        .map(|r| serialize_area_row(areas, r))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Row `r` of `areas` as a string: `"a . b"`.
pub(crate) fn serialize_area_row(areas: &GridTemplateAreas, r: usize) -> String {
    let row = areas.rows()[r]
        .iter()
        .map(|cell| cell.as_deref().unwrap_or("."))
        .collect::<Vec<_>>()
        .join(" ");
    rdom_core::css_syntax::serialize_string(&row)
}
