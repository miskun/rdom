//! Tile 18 — grid layout.
//!
//! Spec: CSS Grid 2 §7.4 (`grid-template`: areas, row sizes, line names),
//! §7.2.3 (`repeat(auto-fill / auto-fit)`: as many tracks as fit; `auto-fit`
//! collapses the empty ones — 0 wide, their gutters with them), §7.2.4
//! (`fr`; `1fr` is `minmax(auto, 1fr)`), §8.3 / §8.5 (line-based placement,
//! `-1` the last explicit line; auto-placement, `dense` filling the
//! earliest hole, implicit rows of `grid-auto-rows`), §9 (a subgrid's
//! padding adds to its edge items' contributions to the parent's tracks),
//! §9.1 / CSS 2.1 §10.1 (an absolutely positioned child of a grid is
//! placed against its grid area), §10 (`justify-self` / `align-self`;
//! `auto` margins take the free space), §10.5 (`justify-content:
//! space-between` spaces the tracks, and a spanning item's area takes the
//! space between them), §11 (track sizing: a fixed track's size; intrinsic
//! base sizes from min-content; a flexible track's growth limit set to its
//! base size after §11.5; §11.6 "maximize tracks" growing `minmax()` and
//! `fit-content()` tracks to their limits — `fit-content(4)` to min(4,
//! max-content); §11.7 the `fr` tracks taking the rest — a `1fr` track
//! held at its item's min-content, which exceeds its share, as an
//! inflexible track), §6.5 / Appendix E (grid items with `z-index` stack),
//! §5.1 (an `inline-grid` is an atomic inline, on its first row's
//! baseline), §7.1 (`direction: rtl` runs the columns from the right).
//! DIVERGENCES §1 "Grid tracks are whole cells", "Alignment free space is
//! shared in whole cells" (`auto` margins and `center` round the leading
//! space down).
//!
//! Derivation (three flex bands, items 2 apart; rows 0, 4, 9):
//!
//! - `.g1` (x 0): columns `[a] 4 [b] 8 [c]`, rows 1 and 2; `h` across row
//!   0 (teal), `s` the 4 × 2 under it (olive), `M` placed by the line
//!   names `b / c`, `mid / bot` — the 8 × 2 beside it (navy).
//! - `.g2` (x 14, 20 wide): `2 1fr minmax(2, 3) fit-content(4)` → base
//!   sizes 2, 1 (`b`), 2, 2 (`dd dd`'s min-content); maximize grows the
//!   third to 3 and the fourth to 4 (its max-content 5 capped at 4); the
//!   `fr` track takes 20 − 9 = 11. `dd dd` wraps in 4: the row is 2 tall
//!   and every item stretches to it.
//! - `.g4` (x 36): three 2-wide columns, rows of 1, `dense`: `f` (`1 / -1`,
//!   row 3) is placed first; `a` (span 2) on row 0, `b` (span 2) does not
//!   fit beside it — row 1; `c` fills the hole at row 0, column 3.
//! - `.g5` (x 44): the subgrid spans both `auto` columns with `padding: 0
//!   1`: column 1 is 1 + 3 (`aaa`), column 2 1 (`b`) + 1; the subgrid
//!   navy over x 44–49 with `aaa` at 45 and `b` at 48; `x` and `y` on the
//!   parent's next row in the same columns.
//! - `.g8` (x 52): `A` (`z-index: 2`) over `B` (1) in the one cell: red,
//!   `A`.
//! - `.g3` (x 0 and 13, 11 wide, `justify-content: end`): `auto-fill` makes
//!   three 3-wide tracks (9), 2 free: the items at x 2 and 5; `auto-fit`
//!   collapses the empty third: 6, 5 free — x 18 and 21.
//! - `.g6` (x 26): bordered, areas `p q` of 3 × 2: `p` at x 27, and the
//!   absolutely positioned `Q` with `inset: 0` filling area `q`, x 30–32,
//!   rows 5–6 (teal).
//! - `.g7` (x 36 and 46, 8 wide): `1fr 1fr` — `abcdefg`'s min-content 7
//!   exceeds the 4 each share would give, so its track is 7 and the other
//!   1: `x` at 43; `minmax(0, 1fr)` twice — 4 and 4: the word overflows its
//!   4 cells, and `x`, painted after it in its own cell at 50, replaces
//!   its `e`: `abcdxfg`.
//! - Row 9: `ab`, the inline grid `X` · `Y` (1-wide columns, gap 1), `cd`:
//!   `abX Ycd`; `.g10` (x 9, `rtl`): column 1 on the right (x 11–12), `1`
//!   aligned to its start, the right — x 12; `2` at x 10. `.g11` (x 15,
//!   10 wide): tracks 2 + 2 + 2 and 4 free shared by `space-between` — the
//!   columns at x 15, 19, 23; `A` (`1 / 3`) takes its two columns and the
//!   gap between, x 15–20, both rows of the 2-row track; `b` aligned to
//!   its area's end, x 24, row 10; `C` (2 wide, `margin: 0 auto`) across
//!   all columns centred: (10 − 2) / 2 = x 19, row 11.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "18",
    spec: &[
        "CSS Grid 2 §5.1, §6.5, §7, §8, §9, §10, §11",
        "CSS 2.1 §10.1, Appendix E",
        "DIVERGENCES §1 grid tracks whole cells, alignment",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('r', "bg #c00000"),
    ],
    grid: r#"
|H             a b          cc dd    a   c    aaab   A     |
|tttttttttttt..ttnnnnnnnnnnnooommmm..ttttoo..nnnnnn..rrrr..|
|S   M                         dd    b       x   y         |
|oooonnnnnnnn..ttnnnnnnnnnnnooommmm..nnnn..................|
|                                    f                     |
|oooonnnnnnnn........................mmmmmm................|
|                                                          |
|..........................................................|
|  1  2            1  2    ┌──────┐  abcdefgx  abcdxfg     |
|..tttttt..........tttttt..................................|
|                          │p  Q  │                        |
|..............................ttt.........................|
|                          │      │                        |
|..............................ttt.........................|
|                          └──────┘                        |
|..........................................................|
|                                                          |
|..........................................................|
|abX Ycd   2 1  A                                          |
|...............tttttt.....................................|
|                        b                                 |
|...............tttttt...n.................................|
|                   C                                      |
|...................oo.....................................|
"#,
};
