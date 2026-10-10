//! Tile 54 — a monolithic box moving whole, cloned decorations (tile
//! 25's left-out cases).
//!
//! Spec: CSS Multi-column 1 §3.4 (two columns of (17 − 1) / 2 = 8, gap 1:
//! x 0 and 9), §7 (`column-fill: auto` fills each column to the height in
//! turn); CSS Fragmentation 3 §4.1 (an `overflow: hidden` box is
//! monolithic: it is not broken — with one row left in the first column it
//! moves whole to the next), §5.4 (`box-decoration-break: clone`: each
//! fragment drawn as a whole box, its border on every fragment);
//! DIVERGENCES §2 "Multi-column layout is laid out in whole cells" (the
//! monolithic boxes, the cloned border taking rows at the break).
//!
//! Derivation: the first article (3 rows): `a1`, `a2` in column 1; the
//! two-row navy box `H1` / `H2` at the top of column 2, `a3` after it. The
//! second (x 19, 4 rows): the bordered box, 6 rows whole, cut after its
//! second line — column 1 its top border, `q1`, `q2` and a cloned bottom
//! border; column 2 (x 28) a cloned top border, `q3`, `q4` and its bottom
//! border.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "54",
    spec: &[
        "CSS Multi-column 1 §3.4, §7; CSS Fragmentation 3 §4.1, §5.4",
        "DIVERGENCES §2 multi-column layout",
    ],
    legend: &[('n', "bg #000080")],
    grid: r#"
|a1       H1        ┌──────┐ ┌──────┐  |
|.........nnnnnnnn.....................|
|a2       H2        │q1    │ │q3    │  |
|.........nnnnnnnn.....................|
|         a3        │q2    │ │q4    │  |
|......................................|
|                   └──────┘ └──────┘  |
|......................................|
"#,
};
