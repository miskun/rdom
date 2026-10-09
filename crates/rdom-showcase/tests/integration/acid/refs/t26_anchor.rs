//! Tile 26 — anchor positioning.
//!
//! Spec: CSS Anchor Positioning 1 §2 (`anchor-name`; `anchor-scope`
//! limits a name to the subtree), §3.1 (`position-area`: the grid of the
//! containing block cut at the anchor's edges; a single keyword `top` is
//! `top span-all`, its default alignment `anchor-center` across and `end`
//! along — the box centred over the anchor and just above it), §3.2
//! (`anchor()`: an edge of the anchor in the containing block's
//! coordinates), §3.4 (`anchor-size()`), §4 (`position-try-fallbacks`:
//! when the base position overflows the inset-modified containing block,
//! the first `@position-try` option that fits), §5
//! (`position-visibility: anchors-visible` hides the box while its anchor
//! is clipped out of view; `always` shows it). DIVERGENCES §2 "Anchor
//! positioning is laid out in whole cells, with four simplifications"
//! (a `position-area` across the anchor centres with the leading space
//! rounded down; `position-visibility` hides from paint).
//!
//! Derivation (every box absolutely positioned against the tile):
//!
//! - `A` (navy, x 2–5, row 1): `P1` at `anchor(--a bottom)` = 2,
//!   `anchor(--a right)` = 6, `anchor-size(--a width)` = 4 wide — teal,
//!   x 6–9, row 2.
//! - `button` (x 14–19, row 3): `tip` (3 wide, olive) in `position-area:
//!   top`, centred over the anchor — 14 + (6 − 3) / 2 rounded down = x 15 —
//!   on the row above it, 2.
//! - `D` (x 50–53, row 1): `P4`'s base position `left: anchor(right)` puts
//!   its 6 cells at 54–59, past the tile's 58: the `--left` option
//!   (`right: anchor(left)`) fits — x 44–49, row 1 (maroon).
//! - Two `.li` (x 2 and 8, row 5) each scoping `--item`: each `tp` (green)
//!   right of its own item's anchor — `1` at x 3, `2` at x 9 (without the
//!   scope both would take the last `--item`).
//! - The scroller (x 20, rows 5–6, no bar) holds its anchor `S` 4 rows
//!   down — out of view: `v` (`anchors-visible`, left of it, x 19) is hidden;
//!   `V` (`always`) is shown where the anchor is, right of it: x 26, row 9.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "26",
    spec: &[
        "CSS Anchor Positioning 1 §2–§5",
        "DIVERGENCES §2 anchor positioning",
    ],
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|                                                          |
|..........................................................|
|  A                                         P4    D       |
|..nnnn......................................mmmmmm........|
|      P1       tip                                        |
|......tttt.....ooo........................................|
|              button                                      |
|..........................................................|
|                                                          |
|..........................................................|
|  x1    y2                                                |
|...g.....g................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                          V                               |
|..........................................................|
"#,
};
