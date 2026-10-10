//! Tile 41 — scroll snapping and chaining, at rest (the state step I11
//! starts from).
//!
//! Spec: CSS Flexbox 1 §9 (seven 4-wide boxes, `gap: 1`, `align-items:
//! flex-start`: x 0, 5, 10, 15, 20, 25, 30); CSS Overflow 3 §3 and CSS
//! Scrollbars 1 §3 (every box scrolls, none draws a bar or a gutter); CSS
//! Scroll Snap 1 §5 (a snap container at rest at offset 0 is at its first
//! item's `start` snap position — no initial scroll).
//!
//! Derivation: each box shows its first lines from offset 0 — `m0`–`m2`,
//! `a0`–`a2`, the outer box's two inner scrollers (`c0`, `c1`; `d0`, `d1`),
//! `t0`–`t2`, `r0`–`r2`, `A0`–`A2`, `x0`–`x3`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "41",
    spec: &["CSS Flexbox 1 §9; CSS Overflow 3 §3; CSS Scrollbars 1 §3; CSS Scroll Snap 1 §5"],
    legend: &[],
    grid: r#"
|m0   a0   c0   t0   r0   A0   x0      |
|......................................|
|m1   a1   c1   t1   r1   A1   x1      |
|......................................|
|m2   a2   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};
