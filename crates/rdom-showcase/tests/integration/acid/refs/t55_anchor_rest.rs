//! Tile 55 — `most-height`, `anchor-center`, `flip-block` (tile 26's
//! left-out cases).
//!
//! Spec: CSS Anchor Positioning 1 §3.1 (`position-area: bottom` — the area
//! under the anchor, across all columns, the box centred on the anchor
//! and hugging it), §4 (`position-try-fallbacks: flip-block` — the area
//! flipped to `top`; `position-try-order: most-height` sorts the base and
//! its options by their inset-modified containing block's height, so the
//! taller one is tried first), §3.4 (`justify-self: anchor-center`: the
//! box centred on its anchor), §3.2 (`anchor(bottom)`); HTML §6.12 (the
//! load script shows the popover; its UA `position: fixed` makes the
//! viewport its containing block, its UA border solid); DIVERGENCES §2
//! "Anchor positioning is laid out in whole cells".
//!
//! Derivation (the tile is the absolute boxes' containing block, page
//! rows 40–49):
//!
//! - Anchor `A` (navy, x 2–5, row 6): `bottom` would have rows 7–9 (3),
//!   its flip rows 0–5 (6): `most-height` tries the flip first, and the 2
//!   rows fit — `MH` (teal) at rows 4–5, x 2–5.
//! - Anchor `B` (navy, x 20–22, row 1): `C` (maroon, 5 wide) under it,
//!   centred — its centre 21.5, so x 19–23, row 2.
//! - `[ pp ]` (x 30, row 8 — page row 48): its popover (7 × 3 with its
//!   border) below would end on page row 51, past the 50-row viewport:
//!   `flip-block` puts it above — tile rows 5–7, from the button's left
//!   edge.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "55",
    spec: &[
        "CSS Anchor Positioning 1 §3.1, §3.2, §3.4, §4; HTML §6.12",
        "DIVERGENCES §2 anchor positioning",
    ],
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('m', "bg #800000"),
        ('B', "fg #1e90ff bold"),
    ],
    grid: r#"
|                                                          |
|..........................................................|
|                    B                                     |
|....................nnn...................................|
|                   C                                      |
|...................mmmmm..................................|
|                                                          |
|..........................................................|
|  MH                                                      |
|..tttt....................................................|
|                              ┌─────┐                     |
|..tttt....................................................|
|  A                           │alpha│                     |
|..nnnn....................................................|
|                              └─────┘                     |
|..........................................................|
|                              [ pp ]                      |
|..............................BBBBBB......................|
|                                                          |
|..........................................................|
"#,
};
