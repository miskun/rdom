//! Tile 39 — smooth scrolling, at rest (the state step I8 starts from).
//!
//! Spec: CSS Overflow 3 §3 (`overflow: auto` boxes, both scrolled to
//! their origin); CSS Scrollbars 1 §3 (`scrollbar-width: none`: no
//! scrollbar, no gutter, the box still scrolls).
//!
//! Derivation: the outer box (10 × 5) shows `top`, the inner box's first
//! three lines on navy (6 wide) and `o2`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "39",
    spec: &["CSS Overflow 3 §3; CSS Scrollbars 1 §3"],
    legend: &[('n', "bg #000080")],
    grid: r#"
|top                                   |
|......................................|
|a0                                    |
|nnnnnn................................|
|a1                                    |
|nnnnnn................................|
|a2                                    |
|nnnnnn................................|
|o2                                    |
|......................................|
"#,
};
