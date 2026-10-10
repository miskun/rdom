//! Tile 53 — overflowing text over a float, a cleared first child's
//! margin (tile 19's left-out cases).
//!
//! Spec: CSS 2.1 Appendix E (a stacking context paints floats at step 5,
//! after block backgrounds and before in-flow inline content at step 7:
//! text overflowing its block is drawn over a float), §9.5 (a float at the
//! right of its 12-wide parent, x 8–11; the 6-wide paragraph's line box
//! does not reach it, so it is not shortened), §16.6.1 (`nowrap`: the word
//! overflows), §8.3.1 / §9.5.2 (a first in-flow child with clearance does
//! not collapse its top margin through its parent; clearance puts it below
//! the float — the greater of its hypothetical position, 2, and the
//! float's bottom, 3); DIVERGENCES §2 "Margins collapse in block flow"
//! (C8G-CLEARANCE-COLLAPSE).
//!
//! Derivation: row 0 `overflowing` from x 0, its `ing` (x 8–10) over the
//! float's maroon, which also fills x 11; row 1 the spacer; the parent
//! from row 2, not moved by the child's margin: its float `F` (teal, 2 ×
//! 3) on rows 2–4, `x` on row 5.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "53",
    spec: &[
        "CSS 2.1 Appendix E, §8.3.1, §9.5, §9.5.2, §16.6.1",
        "DIVERGENCES §2 margin collapsing",
    ],
    legend: &[('m', "bg #800000"), ('t', "bg #008080")],
    grid: r#"
|overflowing                           |
|........mmmm..........................|
|                                      |
|......................................|
|F                                     |
|tt....................................|
|                                      |
|tt....................................|
|                                      |
|tt....................................|
|x                                     |
|......................................|
"#,
};
