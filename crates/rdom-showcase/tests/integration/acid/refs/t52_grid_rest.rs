//! Tile 52 — a baseline row group and an `aspect-ratio` item (tile 18's
//! left-out cases).
//!
//! Spec: CSS Grid 2 §6.2 (`align-self: normal` behaves as `start` for an
//! item with a preferred aspect ratio), §10 / CSS Box Alignment 3 §9 (the
//! row's `baseline` items share a baseline: the one whose first line sits
//! lower — two rows of padding — sets it, the other is shifted down to it,
//! not stretched); CSS Sizing 4 §5.1 (`aspect-ratio`: the automatic height
//! from the definite width, 6 / 3 = 2); DIVERGENCES §2 "A flex or grid
//! container's baselines are its first and last content rows" (an item's
//! baseline its first line's glyph row), "`aspect-ratio` rounds onto the
//! cell grid".
//!
//! Derivation (columns 4, 4, 6, one apart): `a`'s box (navy) rows 0–2,
//! its text on row 2; `b`'s (teal, 1 row) moved down to row 2; the ratio
//! box (maroon) 6 × 2 at x 10, rows 0–1. The row is 3 tall.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "52",
    spec: &[
        "CSS Grid 2 §6.2, §10; CSS Box Alignment 3 §9; CSS Sizing 4 §5.1",
        "DIVERGENCES §2 baselines, aspect-ratio",
    ],
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('m', "bg #800000"),
    ],
    grid: r#"
|                                      |
|nnnn......mmmmmm......................|
|                                      |
|nnnn......mmmmmm......................|
|a    b                                |
|nnnn.tttt.............................|
"#,
};
