//! The acid tiles, one module each, in `ACID.md`'s numbering.

use super::Tile;

mod t01_cascade;

/// Every tile, in `ACID.md` order.
pub const TILES: &[&Tile] = &[&t01_cascade::TILE];
