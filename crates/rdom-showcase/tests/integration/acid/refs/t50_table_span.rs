//! Tile 50 — wrapped cells beside a row span (tile 14's left-out case).
//!
//! Spec: CSS 2.1 §17.5.2.2 (auto layout: a column as wide as its widest
//! cell's minimum — `aa`, 2 — or its `width`), §17.5.3 (a row as tall as
//! its tallest cell; a cell spanning rows takes their heights and the
//! line between them; `vertical-align: middle`, the UA's on the row
//! groups, HTML §15.3.8), §17.6.2 (collapsed borders: one line between
//! cells — none inside a spanning cell); DIVERGENCES §1 (a border is one
//! cell; junctions in the box-drawing set), §2 (the table formatting
//! context in whole cells; `padding: 0` here).
//!
//! Derivation: columns 1 (`R`) and 2 (`aa bb cc` wraps to `aa` / `bb` /
//! `cc`), lines at x 0, 2, 5; the first row 3 rows (rows 1–3), the second
//! 1 (row 5), lines at rows 0, 4, 6. Row 4's line runs under the second
//! column only — `R` spans across it: `│ ├──┤`. `R` in the middle of its
//! five content rows: row 3; `x` at row 5.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "50",
    spec: &[
        "CSS 2.1 §17.5.2.2, §17.5.3, §17.6.2; HTML §15.3.8",
        "DIVERGENCES §1 borders; §2 tables",
    ],
    legend: &[],
    grid: r#"
|┌─┬──┐                                |
|......................................|
|│ │aa│                                |
|......................................|
|│ │bb│                                |
|......................................|
|│R│cc│                                |
|......................................|
|│ ├──┤                                |
|......................................|
|│ │x │                                |
|......................................|
|└─┴──┘                                |
|......................................|
"#,
};
