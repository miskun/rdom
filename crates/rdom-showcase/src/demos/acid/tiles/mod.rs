//! The acid tiles, one module each, in `ACID.md`'s numbering.

use super::Tile;

mod t01_cascade;
mod t02_specificity;
mod t03_inheritance;
mod t04_selectors;

/// Every tile, in `ACID.md` order.
pub const TILES: &[&Tile] = &[
    &t01_cascade::TILE,
    &t02_specificity::TILE,
    &t03_inheritance::TILE,
    &t04_selectors::TILE,
];
