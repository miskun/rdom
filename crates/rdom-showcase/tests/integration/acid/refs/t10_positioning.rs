//! Tile 10 — positioning.
//!
//! Spec: CSS 2.1 §9.4.3 / CSS Position 3 §3.4 (a relatively positioned
//! box is offset from its place in the flow, which it keeps), §10.1
//! (an absolutely positioned box's containing block is the padding box of
//! its nearest positioned ancestor; a fixed box's is the viewport),
//! §10.3.7 / §10.6.4 (the offsets: `left` / `right` / `top` / `bottom`,
//! `inset` the four at once; with both offsets and an `auto` size the size
//! fills the space between them; with every offset `auto` the box sits at
//! its static position, the place it would have had in the flow); CSS 2.1
//! Appendix E (positioned boxes paint after the in-flow content of their
//! stacking context).
//!
//! Derivation (`b` elements made normal weight):
//!
//! - Row 0: `ab rel cd` lays out as written; `rel` keeps its cells x 3–5
//!   empty and paints 2 right and 1 down, at x 5–7 of row 1.
//! - `.pc` (`margin-top: 1`, so row 2): 30 × 6 inside a border, 32 × 8 —
//!   rows 2–9; its padding box is x 1–30, rows 3–8.
//! - `x` and `y` are its first line (row 3, from x 1); `.a5`, absolutely
//!   positioned with every offset `auto`, is at its static position — after
//!   `x`, at x 2 — and `y` lays out there too (the box takes no room in the
//!   line): `S` paints after the in-flow `y`, over it.
//! - `.a1` (`top: 2; left: 3`): x 1 + 3 = 4, row 3 + 2 = 5.
//! - `.a2` (`right: 1; bottom: 0`, 2 wide by its text): its right edge one
//!   cell inside the padding box's (x 29), its bottom on the last row (8):
//!   x 28–29.
//! - `.a4` (`inset: 1 2 2 20`, `auto` width and height): x 1 + 20 = 21 to
//!   30 − 2 = 28 (8 wide), rows 3 + 1 = 4 to 8 − 2 = 6 (3 tall), teal, `A4`
//!   at its top left.
//! - `.fx` (`position: fixed; top: 13; left: 70`) is placed against the
//!   viewport — the 120 × 50 page — whatever its ancestors: page cell
//!   (70, 13), which is this tile's (10, 8). (Its tile's `overflow: clip`
//!   does not clip it: its containing block is not inside the tile.)

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "10",
    spec: &[
        "CSS 2.1 §9.4.3, §10.1, §10.3.7, §10.6.4, Appendix E",
        "CSS Position 3 §3–§4",
    ],
    legend: &[('t', "bg #008080")],
    grid: r#"
|ab     cd                             |
|......................................|
|     rel                              |
|......................................|
|┌──────────────────────────────┐      |
|......................................|
|│xS                            │      |
|......................................|
|│                    A4        │      |
|.....................tttttttt.........|
|│   A1                         │      |
|.....................tttttttt.........|
|│                              │      |
|.....................tttttttt.........|
|│                              │      |
|......................................|
|│         FX                A2 │      |
|......................................|
|└──────────────────────────────┘      |
|......................................|
"#,
};
