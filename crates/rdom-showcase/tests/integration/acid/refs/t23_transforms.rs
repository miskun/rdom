//! Tile 23 — transforms.
//!
//! Spec: CSS Transforms 1 §2 (a transform other than `none` makes the box
//! a stacking context and the containing block of its absolutely and
//! fixed positioned descendants), §3 (a transform moves the box's
//! rendering and affects no layout — siblings stay where flow put them),
//! §5 `transform-box` (`content-box`: percentages of the content box),
//! CSS Transforms 2 §5.1 (`translate` composes before `transform`;
//! percentages of the reference box); CSS Overflow 3 §2.2 (a transformed
//! descendant's moved box is in its scroll container's scrollable
//! overflow); CSS 2.1 Appendix E (a context's `z-index: -1` children paint
//! after its background). DIVERGENCES §1 "A fractional length rounds onto
//! the grid" (half to even); §2 "A transform moves a box by whole cells,
//! and does nothing else" (the translations summed, rounded once; the
//! rotation an identity; the moved box in its scroll container's
//! overflow), "Stacking contexts form from … a transform", §1 "A
//! scrollbar is one cell" (`thin` with `scrollbar-color`: the track cells
//! in the track colour, the thumb `│` in the thumb colour) and the UA's
//! thumb size `track · viewport / content` (rounded down, at least 1).
//!
//! Derivation (two flex bands; rows 0 and 2):
//!
//! - Row 0: `.edge` (10 wide, teal) at x 0 moved −5: its cells −5–4, the
//!   tile's clip leaving `56789`. `.sib` `abcdefgh` at x 12; `.pnl` (navy,
//!   at x 22) moved −5 to x 17–26 over `fgh` — a stacking context, painted
//!   above the in-flow sibling, which stays.
//! - Row 2 (items 3 apart): `T` (x 0) moved by translateX(2) and
//!   translateY(1), the `rotate(45deg)` between them inert — x 2, row 3;
//!   `A` (x 4) by 2.5 → 2 — x 6; `B` (x 8) by 1.5 → 2 — x 10; `.tb` (x 12,
//!   8 wide with its padding, olive) by 50 % of its 4-cell content box — 2:
//!   x 14–21, its `C` at 16; the `scale(1)` box (x 23, maroon) is a
//!   stacking context, so its `z-index: -1` `Z` (yellow) paints above its
//!   background; the transformed card (x 30, 8 × 2, navy) is the
//!   containing block of its `fixed` badge — `right: 0; top: 0`: x 37; the
//!   `overflow: auto` box (x 41, 6 × 3) whose child is moved 5 rows down:
//!   its scrollable overflow is 6 rows in 3, so it scrolls — the gutter at
//!   x 46, a thumb of 3 · 3 / 6 → 1 at the top, the moved child out of
//!   view.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "23",
    spec: &[
        "CSS Transforms 1 §2, §3, §5; CSS Transforms 2 §5.1",
        "CSS Overflow 3 §2.2; CSS 2.1 Appendix E",
        "DIVERGENCES §1 fractional length, scrollbar; §2 transform, stacking contexts",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('y', "fg #ffff00 bg #800000"),
        ('Y', "fg #ffff00 bg #000080"),
    ],
    grid: r#"
|56789       abcde0123456789                               |
|ttttt............nnnnnnnnnn...............................|
|                                                          |
|..........................................................|
|      A   B     C      Z             !        │           |
|..............oooooooo.ymmm...nnnnnnnt........Y...........|
|  T                                                       |
|..............................nnnnnnnn........n...........|
|                                                          |
|..............................................n...........|
"#,
};
