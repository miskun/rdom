//! The acid tiles, one module each, in `ACID.md`'s numbering.

use super::Tile;

mod t01_cascade;
mod t02_specificity;
mod t03_inheritance;
mod t04_selectors;
mod t05_box_model;
mod t06_margins;
mod t07_flex;
mod t08_inline;

/// Every tile, in `ACID.md` order.
pub const TILES: &[&Tile] = &[
    &t01_cascade::TILE,
    &t02_specificity::TILE,
    &t03_inheritance::TILE,
    &t04_selectors::TILE,
    &t05_box_model::TILE,
    &t06_margins::TILE,
    &t07_flex::TILE,
    &t08_inline::TILE,
];
