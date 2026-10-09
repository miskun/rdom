//! Tile 19 — floats.
//!
//! Spec: CSS 2.1 §9.5 (a float shifts left / right until its margin edge
//! touches its containing block or another float; line boxes beside
//! floats are shortened), §9.5.1 rules 2–3 and 7–8 (a left float right of
//! earlier left floats or below them, never overlapping a right float;
//! placed as high as possible — the third float, too wide beside the
//! first two, goes down to where it fits), rule 6 (a float's top no higher
//! than the current line box: a float met mid-line goes on that line, the
//! line's content moved past it), §9.5 "a line box … shortened … if a
//! shortened line box is too small to contain any content, then it is
//! shifted downward", §9.5.2 (`clear`), §9.4.1 / §10.6.7 (a block
//! formatting context root — `flow-root`, `overflow: hidden` — contains
//! its floats, and its border box does not overlap a float in its own
//! formatting context: it is placed beside it), Appendix E (a block's
//! background, step 4, under the floats, step 5, under the inline content,
//! step 7); CSS Logical 1 §2.2 (`float: inline-start` is `right` under
//! `rtl`); CSS Pseudo-Elements 4 §2 (a `::before` floats like an element).
//! DIVERGENCES §2 "Floats follow CSS 2.1 §9.5 with three simplifications"
//! (none of them reached here), "No bidirectional reordering" (an `rtl`
//! line keeps its logical order, aligned to the right).
//!
//! Derivation (three columns: x 0 (20 wide), 22 (16), 40 (14)):
//!
//! - x 0: `L` (left, 3 × 2, teal) and `R` (right, 3 × 1, navy); `F3`
//!   (left, 16 wide) does not fit between them on row 0 (14 cells) and
//!   goes to row 1, right of `L` (x 3–18), where `R` has ended. Row 0's
//!   line: x 3–16 — `one two three`; row 1's: x 19 alone, too small for
//!   `four` — the line moves down; row 2 is clear: `four five six seven`.
//!   Row 3: `aa `, then the float `M` (2 × 1, maroon) goes on this line, at
//!   its left, and the line's text follows it: `aa bb cc` from x 2. Row 4:
//!   the float `W` (3 wide) leaves 17 cells, too few for the 18-letter word,
//!   which moves below it: row 5.
//! - x 22: the `flow-root` box (navy) contains its 2 × 3 float (teal) —
//!   3 rows tall — `x` beside it. Row 3: the float `O` (3 × 2, olive) and
//!   beside it the `overflow: hidden` box (maroon, x 25–37) — a formatting
//!   context root, not overlapping the float; its parent ends with it.
//!   Row 4: the next block's green background runs under `O`'s second row
//!   (the float paints over it), its `ppp` beside it from x 25. Rows 5–6:
//!   the float `Q`; the `clear: left` paragraph `z` below it, row 7.
//! - x 40: `rtl` — the `inline-start` float `I` on the right (x 52–53, its
//!   `I` at its own start, the right), `ab cd` right-aligned beside it
//!   (x 47–51). Rows 1–2: the clearfix box (maroon) as tall as its float
//!   `C` (navy); `after` below it, row 3. Row 4: the media float `M` and its
//!   `margin-right: 1`: `txt` from x 43. Row 5: `ab` and the floated
//!   `::before` `F` (green) at the right, x 53. Row 6: the float `D` from
//!   inside `display: contents` floats in the parent's flow, `tail` beside
//!   it.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "19",
    spec: &[
        "CSS 2.1 §9.4.1, §9.5, §9.5.1, §9.5.2, §10.6.7, Appendix E",
        "CSS Logical 1 §2.2; CSS Pseudo-Elements 4 §2",
        "DIVERGENCES §2 floats, no bidirectional reordering",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('v', "bg #008000"),
    ],
    grid: r#"
|L  one two three R    B x                      ab cd I    |
|ttt..............nnn..ttnnnnnnnnnnnnnn..............tt....|
|   F                                    C                 |
|tttoooooooooooooooo...ttnnnnnnnnnnnnnn..nnmmmmmmmmmmmm....|
|four five six seven                                       |
|......................ttnnnnnnnnnnnnnn..nnmmmmmmmmmmmm....|
|M aa bb cc            O  hid            after             |
|mm....................ooommmmmmmmmmmmm....................|
|W                        ppp            M  txt            |
|ttt...................ooovvvvvvvvvvvvv..oo................|
|abcdefghijklmnopqr    Q                 ab           F    |
|......................tt.............................v....|
|                                        Dtail             |
|......................tt................n.................|
|                      z                                   |
|..........................................................|
"#,
};
