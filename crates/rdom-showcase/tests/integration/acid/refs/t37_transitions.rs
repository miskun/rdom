//! Tile 37 — transitions on the clock, at rest (the state step I6
//! starts from).
//!
//! Spec: CSS Transitions 1 §3 (a transition starts only on a change of
//! a computed value: at rest nothing runs); CSS Flexbox 1 §9 (row 0 a
//! flex row with `gap: 1` of 4-wide swatches; row 1 with `gap: 0`; row 3
//! with `gap: 1`, `align-items: flex-start`); HTML §15.5.20 /
//! DIVERGENCES §2 (a closed `details`: its summary `▸ s`, its
//! `::details-content` `content-visibility: hidden` and here `height: 0`
//! — no rows).
//!
//! Derivation: row 0 the four swatches black at x 0–3, 5–8, 10–13,
//! 15–18 and `log:` at x 20; row 1 `abc`; row 2 `pad`; row 3 the 4 × 1
//! navy box and `▸ s` at x 5 (the `details`, 6 wide, one cell after
//! it); row 4 `end`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "37",
    spec: &[
        "CSS Transitions 1 §3; CSS Flexbox 1 §9",
        "HTML §15.5.20; DIVERGENCES §2 details slot, summary triangle",
    ],
    legend: &[('K', "bg #000000"), ('n', "bg #000080")],
    grid: r#"
|                    log:                                  |
|KKKK.KKKK.KKKK.KKKK.......................................|
|abc                                                       |
|..........................................................|
|pad                                                       |
|..........................................................|
|     ▸ s                                                  |
|nnnn......................................................|
|end                                                       |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};
