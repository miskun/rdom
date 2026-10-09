//! Tile 25 — multi-column layout.
//!
//! Spec: CSS Multi-column 1 §3.4 (the pseudo-algorithm: with `column-count`
//! N and the available width U, column width (U − (N − 1) · gap) / N; with
//! `column-width` W, N = ⌊(U + gap) / (W + gap)⌋), §3.5 (`column-gap:
//! normal`), §4 (a column rule in the middle of each gap between two
//! columns with content, as tall as the columns), §6 (a `column-span: all`
//! element spans every column and splits the flow into column sets before
//! and after it), §7 (`column-fill: balance` — the least height that fits
//! the content; `auto` fills each column to the container's height in
//! turn, the rest in overflow columns beside, in the inline direction);
//! CSS Fragmentation 3 §3.1 (`break-before: column`), §4.4 (`orphans` /
//! `widows`: the least lines a fragment keeps at a break, else the break
//! moves earlier), §5.4 (a box across a break drawn as its fragments,
//! sliced). DIVERGENCES §2 "Multi-column layout is laid out in whole
//! cells" (the width floored, `normal` one cell, a rule one cell in the
//! gap's middle joining the container's border in junction glyphs).
//!
//! Derivation (row 0: three inline-blocks, `vertical-align: top`, 2 apart
//! — x 0, 30, 58 — sized by their own content, so each balances; row 6):
//!
//! - `.a1`, 26 wide inside its border (x 0–27, rows 0–4), three columns of
//!   (26 − 2) / 3 = 8 (x 1, 10, 19), the rules at x 9 and 18 meeting the
//!   border in `┬` / `┴`. Eight one-row lines balance at 3 rows: `d1`,
//!   `d2` and the red paragraph's first line `r1` (its background across
//!   the column, nothing in the gap) in column 1; its second line `r2`,
//!   `d3`, `d4` in column 2; `d5`, `d6` in column 3 (the rule still runs
//!   the columns' height).
//! - `.a2` (x 30), the same flow with a three-line paragraph and
//!   `widows: 3`: at 3 rows the paragraph would leave one or two lines
//!   after a break, so its break moves before it — and then 4 rows are the
//!   least that fit: `d1`, `d2` | `r1`–`r3` (red), `d3` | `d4`–`d6`.
//! - `.a3` (x 58), rules: `s1` | `s2` | `s3` (a 1-row set, its rules), the
//!   spanner `SPAN` (navy) across all 26 cells, then the second set:
//!   `break-before: column` ends column 1 after `t1`, `t2`, so `t3`–`t6`
//!   balance in two rows over columns 2 and 3; its rules on rows 2–3.
//! - `.a4` (row 6): `column-width: 12` in 26: ⌊27 / 13⌋ = 2 columns of
//!   ⌊25 / 2⌋ = 12 (x 0, 13); `column-fill: auto` and `height: 2` fill
//!   `u1`, `u2` then `u3`, `u4`; `u5`, `u6` in an overflow column at x 26,
//!   past the box's width.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "25",
    spec: &[
        "CSS Multi-column 1 §3.4, §3.5, §4, §6, §7",
        "CSS Fragmentation 3 §3.1, §4.4, §5.4",
        "DIVERGENCES §2 multi-column layout",
    ],
    legend: &[('r', "bg #c80000"), ('n', "bg #000080")],
    grid: r#"
|┌────────┬────────┬────────┐  d1       r1       d4        s1      │s2      │s3            |
|.......................................rrrrrrrr...........................................|
|│d1      │r2      │d5      │  d2       r2       d5        SPAN                            |
|..........rrrrrrrr.....................rrrrrrrr...........nnnnnnnnnnnnnnnnnnnnnnnnnn......|
|│d2      │d3      │d6      │           r3       d6        t1      │t3      │t5            |
|.......................................rrrrrrrr...........................................|
|│r1      │d4      │        │           d3                 t2      │t4      │t6            |
|.rrrrrrrr.................................................................................|
|└────────┴────────┴────────┘                                                              |
|..........................................................................................|
|                                                                                          |
|..........................................................................................|
|u1           u3           u5                                                              |
|..........................................................................................|
|u2           u4           u6                                                              |
|..........................................................................................|
"#,
};
