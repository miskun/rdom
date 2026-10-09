//! Tile 16 — display and visibility.
//!
//! Spec: CSS Display 3 §2 (the multi-keyword values: `inline flow-root` is
//! an inline-block, `block flow` a block, `block flex` a flex container),
//! §2.5 (`display: none` generates no box; `contents` none for itself, its
//! children taking its place), §2.7 (a flex item is blockified, so a
//! `<span>` item takes a `width`); CSS 2.1 §9.2.1.1 (a block among inline
//! content: the inline runs go into anonymous block boxes), §8.3.1 (a
//! parent with no border or padding lets its first child's top margin
//! collapse through it; `flow-root` establishes a block formatting
//! context, which keeps it inside), §11.2 (`visibility: hidden`: the box
//! takes its space, draws nothing); CSS 2.1 §17.5.5 (a `collapse` row is
//! removed); Flexbox 1 §4.4 / §9.8 (a `collapse` flex item: zero main
//! size when the items are collected into lines, then ignored "entirely
//! (as if they were `display: none`)" but for its cross size, kept as a
//! strut in its line), CSS Box Alignment 3 §8 (`gap` between adjacent
//! items — so not around the ignored one); HTML's `hidden` attribute (`display: none`), §15.5.20
//! (`details`: the summary alone while closed, the content too while
//! open); Selectors 4 §13.2 (`:empty`). DIVERGENCES §2 "`<summary>`'s
//! disclosure triangle is its `::before`" (`▸ ` / `▾ `), "A `<details>`'s
//! content slot is a box of the box tree" (closed, it takes no rows); the
//! UA table cells' `padding: 0 1`.
//!
//! Derivation, row by row:
//!
//! - 0: `a`, the `display: none` `X` gone, `b `, the hidden `XX` keeping
//!   two blank cells, `c `, the `[hidden]` span gone, `d`: `ab   c d`.
//! - 1: `x`, the `inline flow-root` `IB` on teal, `y`, the `inline-flex`
//!   `FL` (its text an anonymous item) on navy, `z`.
//! - 2–3: the `block flow` span `bl` is a block, so `in` goes into an
//!   anonymous block below it.
//! - 4–5: the `block flex` row, gap 1: the `contents` div's `p` and `q`
//!   are items (x 0, 2); the blockified `r` 3 wide on olive (x 4–6); the
//!   collapsed `s` takes no width but its height 2 stands as the line's
//!   strut, so the line — and every stretched item, `r`'s olive included —
//!   is 2 rows; past line collection it is ignored as `display: none`
//!   would be, so one gap separates `r` and `t`: x 8. (First derived with
//!   a gap on each side of `s` — x 9 — reading §4.4's "it stays an item";
//!   §9.8 step 3's "ignore the collapsed items entirely" is the rule for
//!   the main-axis placement, where the gaps are.)
//! - 6–7: `.nr`'s child margin collapses through it: the margin (row 6)
//!   is outside, `.nr`'s maroon is row 7 alone, `u` on it.
//! - 8–9: `.fr` (`flow-root`) keeps the same margin inside: navy on rows
//!   8–9, `v` on row 9.
//! - 10: the closed `details`: `▸ S`, its content in no row.
//! - 11–12: the open one: `▾ O`, then `k`.
//! - The `display: none` block takes no row; 13: the `:empty` box, 2 × 1
//!   teal.
//! - 14–15: the table's middle row is `collapse`: `1`, then `3` (cell
//!   padding 1: x 1).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "16",
    spec: &[
        "CSS Display 3 §2; CSS 2.1 §8.3.1, §9.2.1.1, §11.2, §17.5.5",
        "Flexbox 1 §4.4; Box Alignment 3 §8; HTML §15.5.20; Selectors 4 §13.2",
        "DIVERGENCES §2 summary triangle, details content slot",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
    ],
    grid: r#"
|ab   c d                                                  |
|..........................................................|
|xIByFLz                                                   |
|.tt.nn....................................................|
|bl                                                        |
|..........................................................|
|in                                                        |
|..........................................................|
|p q r   t                                                 |
|....ooo...................................................|
|                                                          |
|....ooo...................................................|
|                                                          |
|..........................................................|
|u                                                         |
|mmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm|
|                                                          |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|v                                                         |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|▸ S                                                       |
|..........................................................|
|▾ O                                                       |
|..........................................................|
|k                                                         |
|..........................................................|
|                                                          |
|tt........................................................|
| 1                                                        |
|..........................................................|
| 3                                                        |
|..........................................................|
"#,
};
