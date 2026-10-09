//! Tile 7 — flex layout.
//!
//! Spec: CSS Flexbox 1 §7.1 (`flex` shorthand), §9.7 (resolving flexible
//! lengths: free space shared by `flex-grow`, overflow taken by
//! `flex-shrink` × base size, a violated min-content minimum frozen and
//! the rest redistributed), §4.5 (automatic minimum: the content size
//! suggestion), §5.1 (`row-reverse` / `column-reverse` start at the far
//! edge), §5.2 / §9.3 (`flex-wrap` lines; `wrap-reverse` stacks them from
//! the cross end), §5.4 (`order`), §8.1 (`auto` margins absorb free space
//! first), §8.2 (`justify-content`), §8.3 / §9.4 (`align-self`: `stretch`
//! fills the line, `center`, `flex-end`, `baseline` aligns the first
//! baselines), §4 (a run of text is an anonymous item; `::before` /
//! `::after` are items); CSS Box Alignment 3 §8 (gaps); DIVERGENCES §1
//! "Alignment free space is shared in whole cells" (grow shares roll so
//! they sum to the whole; `center` rounds its leading space down;
//! `space-between` places at rolling positions).
//!
//! Derivation (rows; items navy, teal, purple by DOM position; text at
//! each item's start):
//!
//! - 0 `.f1`, 31 wide: bases 4, 4 and a fixed 5 leave 18 free, grown 1 : 2
//!   — 6 and 12 — so `a` is 10 (x 0–9), `b` 16 (x 10–25), `c` 5 (x 26–30).
//! - 1 `.f2`, 20 wide: bases 15 and 15 overflow by 10, shrunk by
//!   shrink × base, 15 : 60 — 2 and 8 — so 13 and 7; but `longword`'s
//!   minimum is its min-content width, 8: it is frozen at 8 and the other
//!   takes the rest, 12. `aaaa bbbb` fits 12 on one line.
//! - 2 `.f3`, `row-reverse`, 30 wide, three 3-cell items packed from the
//!   right: item 1 at x 27, 2 at x 24, 3 at x 21.
//! - 3 `.f4`, `space-between`, 20 wide, three 2-cell items: 14 free, 7 per
//!   gap: x 0, 9, 18.
//! - 4 `.f4c`, `center`, 21 wide: 15 free, leading 7 (rounded down): x 7,
//!   9, 11.
//! - 5 `.f5`, 20 wide: the middle item's two `auto` margins take the 14
//!   free cells, 7 each: x 0, 9, 18.
//! - 6–8 `.f6`, 3 rows tall, `align-items: flex-start`: `a` stretches over
//!   the 3 rows; `b` centred, (3 − 1) / 2 = row 7; `c` at the end, row 8;
//!   `e` (`padding-top: 1`) and `f` share a baseline: e's first line is a
//!   row below its top, so f moves down a row — e on rows 6–7 with its `e`
//!   on row 7, f on row 7.
//! - 9–11 `.f7`, `column-reverse`, 4 wide: items stretch to 4 cells, item 1
//!   at the bottom (row 11), 3 on top (row 9).
//! - 12–14 `.f8`, 14 wide, `wrap`, gaps 2 / 1: `order: -1` puts item 3
//!   first; 6 + 2 + 6 = 14 fills a line: line 1 is 3 (x 0) and 1 (x 8),
//!   row 12; the row gap is row 13; line 2 is item 2, row 14.
//! - 15–16 `.f8r`, `wrap-reverse`: lines 1–2 then 3, stacked from the
//!   bottom: line 1 on row 16, item 3 on row 15.
//! - 17 `.f9`, `gap: 1`: four items — `::before` `B`, the anonymous `anon`,
//!   the `b` element `el` (bold), `::after` `A`: x 0, 2, 7, 10.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "7",
    spec: &[
        "CSS Flexbox 1 §4, §5, §7.1, §8, §9.3, §9.4, §9.7",
        "CSS Box Alignment 3 §8",
        "DIVERGENCES §1 alignment free space in whole cells",
    ],
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('p', "bg #800080"),
        ('B', "bold"),
    ],
    grid: r#"
|a         b               c                               |
|nnnnnnnnnnttttttttttttttttppppp...........................|
|aaaa bbbb   longword                                      |
|nnnnnnnnnnnntttttttt......................................|
|                     3  2  1                              |
|.....................ppptttnnn............................|
|1        2        3                                       |
|nn.......tt.......pp......................................|
|       c c c                                              |
|.......nnttpp.............................................|
|a        b        c                                       |
|nn.......tt.......pp......................................|
|a                                                         |
|nnn......nnn..............................................|
|   b     e  f                                             |
|nnnttt...nnnttt...........................................|
|      c                                                   |
|nnn...ppp.................................................|
|3                                                         |
|pppp......................................................|
|2                                                         |
|tttt......................................................|
|1                                                         |
|nnnn......................................................|
|3       1                                                 |
|pppppp..nnnnnn............................................|
|                                                          |
|..........................................................|
|2                                                         |
|tttttt....................................................|
|3                                                         |
|pppppp....................................................|
|1       2                                                 |
|nnnnnn..tttttt............................................|
|B anon el A                                               |
|.......BB.................................................|
"#,
};
