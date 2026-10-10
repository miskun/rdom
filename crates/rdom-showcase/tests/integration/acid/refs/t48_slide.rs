//! Tile 48 — a sliding panel, at rest (the state step I20 starts from).
//!
//! Spec: CSS Transforms 2 §5.1 (`translate: -100% 0`: the panel moved its
//! own width left — wholly off the tile's left edge, clipped), CSS
//! Transforms 1 §3 (the same for `transform: translateX(-100%)`; neither
//! moves its siblings); the UA's button (`[ … ]`, bold accent); CSS
//! Flexbox 1 §9 (`gap: 2`). DIVERGENCES §2 "A transform moves a box by
//! whole cells" (the `rotate()` spinner draws nothing different).
//!
//! Derivation: row 0 `[ go ]`; row 1 empty (the panel at x −10–−1); row
//! 2 the keyframed box likewise off the tile, the spinner `S` at x 12.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "48",
    spec: &[
        "CSS Transforms 1 §3; CSS Transforms 2 §5.1; CSS Flexbox 1 §9",
        "DIVERGENCES §2 transform",
    ],
    legend: &[('B', "fg #1e90ff bold")],
    grid: r#"
|[ go ]                                                    |
|BBBBBB....................................................|
|                                                          |
|..........................................................|
|            S                                             |
|..........................................................|
"#,
};
