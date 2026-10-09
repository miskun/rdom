//! Tile 11 — stacking contexts.
//!
//! Spec: CSS 2.1 Appendix E and §9.9.1 (a stacking context paints its
//! negative `z-index` children, then its in-flow content, then its
//! `z-index: auto` / `0` positioned descendants in tree order, then the
//! positive ones by `z-index`; a child stacking context is painted whole,
//! atomically, at its own level — nothing inside it rises above a sibling
//! context with a higher `z-index`); CSS Position 3; CSS Backgrounds 3
//! §3 (an opaque background covers what is beneath it — DIVERGENCES §1: a
//! cell shows one glyph).
//!
//! Derivation. `.sc` is positioned with `z-index: auto` — no stacking
//! context — so its children stack in the tile's. In-flow, row 0 reads
//! `INFLOW TEXT` (x 0–10). Then, in paint order:
//!
//! 1. `.n` (`z-index: -1`, red, x 0–5, rows 0–1) — first, under the text:
//!    `INF` keeps the cells, on red; its own `N` is covered.
//! 2. The in-flow text.
//! 3. `.z0` (`z-index: 0`, green, x 3–6, rows 0–1) over the text (`LOW `)
//!    and over `.n`: `Z` at x 3, green blanks after it.
//! 4. `.ctx` (`z-index: 1`, purple, x 12–19, rows 1–3), with its `C`
//!    (x 12) and its `.inner` (`z-index: 99`, yellow, x 10–13 of row 1).
//! 5. `.p2` (`z-index: 2`, blue, x 8–13, rows 0–1) over everything: `EXT`
//!    and the start of `.ctx`, `C` and the whole of `.inner` included —
//!    `.inner`'s 99 counts only inside `.ctx`. `P2` at x 8.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "11",
    spec: &[
        "CSS 2.1 Appendix E, §9.9.1",
        "CSS Position 3",
        "DIVERGENCES §1 one glyph per cell",
    ],
    legend: &[
        ('r', "bg #c00000"),
        ('G', "bg #00a000"),
        ('B', "bg #0000c0"),
        ('p', "bg #800080"),
    ],
    grid: r#"
|INFZ   TP2                            |
|rrrGGGG.BBBBBB........................|
|                                      |
|rrrGGGG.BBBBBBpppppp..................|
|                                      |
|............pppppppp..................|
|                                      |
|............pppppppp..................|
"#,
};
