//! Tile 44 — popover entry and exit, at rest (the state step I16 starts
//! from).
//!
//! Spec: CSS Backgrounds 3 §3.2 (the tile's black background over every
//! cell); CSS Position 3 §3 (the maroon box absolute at (4, 2), 2 × 1);
//! HTML §6.12 (no popover showing: both `display: none`); the UA's
//! buttons (`[ … ]`, bold accent).
//!
//! Derivation: row 0 `[ f ]` (x 0–4) and `[ g ]` (x 6–10) on black; the
//! maroon box at x 4–5 of row 2; every other cell black.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "44",
    spec: &["CSS Backgrounds 3 §3.2; CSS Position 3 §3; HTML §6.12, §15.5"],
    legend: &[
        ('K', "bg #000000"),
        ('b', "fg #1e90ff bg #000000 bold"),
        ('m', "bg #800000"),
        ('n', "fg #c8c8c8 bg #000080"),
        ('h', "fg #646464 bg #000040"),
        ('e', "fg #a46464 bg #400040"),
    ],
    grid: r#"
|[ f ] [ g ]                           |
|bbbbbKbbbbbKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKmmKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
"#,
};
