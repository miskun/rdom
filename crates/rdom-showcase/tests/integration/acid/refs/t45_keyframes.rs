//! Tile 45 — keyframes on the clock, at rest (the state step I17
//! starts from).
//!
//! Spec: CSS Animations 1 §3 (no element names an animation until
//! `.run` is added: nothing runs); CSS Flexbox 1 §9 (row 0, 4-wide boxes,
//! `gap: 1`); CSS Cascade 4 §6.1 (`width: 3 !important`).
//!
//! Derivation: six grey `#404040` swatches at x 0, 5, 10, 15, 20, 25;
//! row 1 a 1-wide navy box, row 2 a 3-wide one, row 3 a 4-wide teal
//! one; rows 4 and 5 `log:`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "45",
    spec: &["CSS Animations 1 §3; CSS Flexbox 1 §9; CSS Cascade 4 §6.1"],
    legend: &[
        ('G', "bg #404040"),
        ('n', "bg #000080"),
        ('T', "bg #008080"),
    ],
    grid: r#"
|                                                          |
|GGGG.GGGG.GGGG.GGGG.GGGG.GGGG.............................|
|                                                          |
|n.........................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTT......................................................|
|log:                                                      |
|..........................................................|
|log:                                                      |
|..........................................................|
"#,
};
