//! Tile 9c — first line, first letter and highlights.
//!
//! Spec: CSS Pseudo-Elements 4 §2.2 (`::first-line` styles the first
//! formatted line of a block container — here inside its first block child
//! — and only the properties of §2.2.1, `color`, `text-transform` and
//! `letter-spacing` among them; what does not fit is laid out on the next
//! line without them), §2.3 (`::first-letter` takes the first letter and
//! the punctuation before it; floated, it is a box beside the lines);
//! CSS Text 3 §9.2 (`letter-spacing` after every unit but the last on a
//! line, DIVERGENCES §1 "Letter and word spacing are whole blank cells"),
//! §2.1 (`uppercase`); CSS 2.1 §9.5 (lines beside a float are shortened),
//! CSS Inline 3 §5.1 (the drop cap's `line-height: 2`, DIVERGENCES §2
//! "`line-height` is whole rows"); CSS Custom Highlight API 1 §3–§5 (a
//! registered highlight's ranges, here across three text nodes, painted
//! with `::highlight()`'s `color`, `background-color` and decorations only).
//!
//! Derivation (one flex band, two rows; items a cell apart):
//!
//! - x 0, 10 wide: the first line is `AB CD` spaced: `A`, blank, `B`,
//!   blank, the space, blank, `C`, blank, `D` — 9 cells, the last unit
//!   without its blank. `EF` would need 3 more: the line breaks, and `ef
//!   gh` is the second line, lowercase, uncoloured, unspaced.
//! - x 11, 12 wide: the first letter is `“A` (punctuation and letter),
//!   floated left, bold red, `line-height: 2` (2 rows, its glyph row on
//!   the first) and `padding-right: 1`: a 3 × 2 box at x 11–13. The rest,
//!   `bc def ghi jkl`, wraps in the 9 cells beside it: `bc def` / `ghi jkl`.
//! - x 24: `a needle b`, with `ed` in a `b`: the `search` highlight covers
//!   `needle` across the three text nodes — yellow on navy, underlined;
//!   `ed` keeps its bold, which a highlight does not change.
//!
//! The `::first-letter:hover` rule does not apply: nothing is hovered.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "9c",
    spec: &[
        "CSS Pseudo-Elements 4 §2.2, §2.2.1, §2.3",
        "CSS Text 3 §2.1, §9.2; CSS 2.1 §9.5",
        "CSS Custom Highlight API 1 §3–§5",
        "DIVERGENCES §1 letter spacing; §2 line-height",
    ],
    legend: &[
        ('g', "fg #00a000"),
        ('R', "fg #c00000 bold"),
        ('h', "fg #ffff00 bg #000080 underline"),
        ('H', "fg #ffff00 bg #000080 underline bold"),
    ],
    grid: r#"
|A B   C D  “A bc def    a needle b    |
|g.g...g.g..RR.............hhHHhh......|
|ef gh         ghi jkl                 |
|......................................|
"#,
};
