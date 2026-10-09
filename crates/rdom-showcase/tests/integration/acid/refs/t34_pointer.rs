//! Tile 34 — hover, press and `pointer-events`, at rest (the state steps
//! I1, I2 and I10 start from).
//!
//! Spec: CSS Flexbox 1 §9 (each row a flex row with `gap: 1`: items at
//! their max-content widths, one cell apart); CSS 2.1 §9.4.2 (the `.p`
//! box's inline content `ab` + `kid` + `cd` on one line); CSS Position 3
//! §3 (the overlays are absolute, out of flow: they move nothing, and
//! with no background, border or content they draw nothing — CSS 2.1
//! Appendix E paints only what a box has); Selectors 4 §9.2 / §9.4 (no
//! pointer yet: nothing matches `:hover` or `:active`).
//!
//! Derivation: row 0 `abkidcd` (x 0–6), `sib` (8–10), `far` (12–14); row
//! 1 `press` (0–4), `log:` (6–9); row 2 `hit` (0–2), `blk` (4–6), the
//! counters `0` at 8 and 10. Every cell in the default style.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "34",
    spec: &[
        "CSS Flexbox 1 §9; CSS 2.1 §9.4.2, Appendix E; CSS Position 3 §3",
        "Selectors 4 §9.2, §9.4",
    ],
    legend: &[],
    grid: r#"
|abkidcd sib far                       |
|......................................|
|press log:                            |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
};
