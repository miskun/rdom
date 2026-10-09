//! Tile 5 — box model.
//!
//! Spec: CSS Box 3 §3–§4 (padding and border around the content box);
//! CSS Sizing 3 §4 (`box-sizing`: `content-box` sizes the content box,
//! `border-box` the border box), §5.2 (`min-*` / `max-*` clamp the used
//! size); CSS Sizing 4 §5 (`aspect-ratio` gives the `auto` height from the
//! width, on the box `box-sizing` names); CSS Values 4 §10.9 (a percentage
//! of the containing block, `calc()` mixing it with a length) with
//! DIVERGENCES §1 "A fractional length rounds onto the grid" (half to
//! even); CSS Backgrounds 3 §4.2 (the border styles; `hidden` has no
//! width), §4.4 / §5 (corners; a non-zero `border-radius` rounds them);
//! CSS Flexbox 1 §9.4, §8.3 (`align-items: flex-start`: each item keeps
//! its own height); DIVERGENCES §1 "A border is one cell wide", "A border
//! corner takes one side's color", "A rounded corner is one arc glyph",
//! §2 "Border-style support … terminal-faithful degradation" (`dashed`
//! straight runs `╌` / `╎`, solid corners).
//!
//! Derivation. Each row is a flex row with a one-cell gap, so the boxes
//! sit left to right at the sums of their border-box widths plus one. The
//! border colour is the text colour, the default, except on `b5` and `b8`.
//!
//! Row 0–2 (first flex row, 58 cells wide):
//! - `b1`: content 6, padding 1 each side, border 1: 10 × 3 at x 0;
//!   `w6` at x 2 (border + padding).
//! - `b2`: `border-box` 25 % of 58 = 14.5, half to even: 14 wide, 3 tall
//!   with the border: x 11–24, double glyphs `╔═╗║╚╝`, `25%` at x 12.
//! - `b3`: `calc(50% - 20)` = 29 − 20 = 9 content, 11 with the border:
//!   x 26–36; `border-radius: 1` draws `╭╮╰╯`.
//! - `b4`: `width: 2` clamped up by `min-width: 5`: 7 with the border at
//!   x 38–44; dashed runs `╌` / `╎`, solid corners.
//! - `b5`: `width: 30` clamped down by `max-width: 4`: 6 at x 46–51. Top
//!   red, right green, bottom blue, left yellow; every corner joins a
//!   horizontal and a vertical side of equal width and style, so it takes
//!   the horizontal side's colour: red on top, blue below.
//!
//! Row 3 is the second row's `margin-top: 1`. Rows 4–9:
//! - `b6`: width 8 and `aspect-ratio: 2` (content box): height 4, so the
//!   box is 10 × 6 (rows 4–9); `ar2` on its first content row.
//! - `b7`: content 5 × 1, padding top 1, right 2, bottom 0, left 3: 12 × 4
//!   at x 11–22; `pad` at x 11 + 1 + 3 = 15 on row 4 + 1 + 1 = 6.
//! - `b8`: top solid green, right double blue, bottom `hidden` (no
//!   border, no row), left dashed yellow: 8 wide (6 + 2), 3 tall (2 + top)
//!   at x 24–31. The top-left corner joins solid and dashed: solid ranks
//!   higher, so green `┌`. The top-right joins solid and double: double
//!   ranks higher, so blue, and the glyph joins a single line from the
//!   left with a double one going down: `╖`. No bottom corners; the side
//!   lines run to the box's last row.
//! - `b9`: `border: hidden` (no border), `height: 5` clamped by
//!   `max-height: 1`: `hid` at x 33, row 4.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "5",
    spec: &[
        "CSS Box 3 §3–§4",
        "CSS Sizing 3 §4, §5.2; Sizing 4 §5",
        "CSS Backgrounds 3 §4.2, §4.4, §5",
        "DIVERGENCES §1 borders, fractional lengths; §2 border styles",
    ],
    legend: &[
        ('r', "fg #c00000"),
        ('g', "fg #00a000"),
        ('b', "fg #0000c0"),
        ('y', "fg #a0a000"),
    ],
    grid: r#"
|┌────────┐ ╔════════════╗ ╭─────────╮ ┌╌╌╌╌╌┐ ┌────┐      |
|..............................................rrrrrr......|
|│ w6     │ ║25%         ║ │calc     │ ╎min  ╎ │max │      |
|..............................................y....g......|
|└────────┘ ╚════════════╝ ╰─────────╯ └╌╌╌╌╌┘ └────┘      |
|..............................................bbbbbb......|
|                                                          |
|..........................................................|
|┌────────┐ ┌──────────┐ ┌──────╖ hid                      |
|........................gggggggb..........................|
|│ar2     │ │          │ ╎sides ║                          |
|........................y......b..........................|
|│        │ │   pad    │ ╎      ║                          |
|........................y......b..........................|
|│        │ └──────────┘                                   |
|..........................................................|
|│        │                                                |
|..........................................................|
|└────────┘                                                |
|..........................................................|
"#,
};
