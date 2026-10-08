//! The table properties' values: `table-layout` (CSS 2.1 §17.5.2) and
//! `caption-side` (§17.4.1).

use super::keyword::parse_keyword;
use crate::layout::{CaptionSide, EmptyCells, TableLayout};
use crate::parse::token::Token;

/// `table-layout`: `auto | fixed`.
pub fn parse_table_layout(value: &[Token]) -> Option<TableLayout> {
    parse_keyword(
        value,
        &[("auto", TableLayout::Auto), ("fixed", TableLayout::Fixed)],
    )
}

/// `caption-side`: `top | bottom` — the table's block-start and block-end
/// sides (CSS Tables 3), so `block-start` / `block-end` are the same two.
pub fn parse_caption_side(value: &[Token]) -> Option<CaptionSide> {
    parse_keyword(
        value,
        &[
            ("top", CaptionSide::Top),
            ("bottom", CaptionSide::Bottom),
            ("block-start", CaptionSide::Top),
            ("block-end", CaptionSide::Bottom),
        ],
    )
}

/// `empty-cells` (§17.6.1.1): `show | hide`.
pub fn parse_empty_cells(value: &[Token]) -> Option<EmptyCells> {
    parse_keyword(
        value,
        &[("show", EmptyCells::Show), ("hide", EmptyCells::Hide)],
    )
}
