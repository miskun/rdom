//! Tile 27 — flow-relative properties.
//!
//! Spec: CSS Logical 1 §2–§6 (each flow-relative property maps to its
//! physical twin by the box's writing mode and `direction`: in
//! `horizontal-tb` block-start / -end are top / bottom, inline-start /
//! -end left / right under `ltr` and right / left under `rtl`; the
//! shorthands set both sides, later declarations win), §4 (the corner
//! radii: `start-start` is top-left under `ltr`, top-right under `rtl`);
//! CSS 2.1 §10.3.3 (an over-constrained block under `rtl` keeps its right
//! margin — the box at its containing block's right edge), §9.4.3
//! (`position: relative` moves the box by its inline-start inset toward
//! the inline end); CSS Overflow 3 §2 (`overflow-inline: hidden` clips
//! the overflowing line — under `rtl` the line overflows to the left, so
//! its last letters show; `overflow-block: clip` the second line).
//! DIVERGENCES §1 "Every box lays out as `horizontal-tb`" (`writing-mode`
//! is kept and changes nothing), §1 "A border is one cell wide; its width
//! picks a glyph weight" and "A border corner takes one side's color"
//! (the wider side, then the heavier style, then the horizontal side; no
//! glyph joins heavy and double, so the dominant side's set draws the
//! cell), §2 "Border-style support … `dashed` … `╏` heavy", §1 "A rounded
//! corner is one arc glyph", §2 "The flow-relative properties share the
//! physical ones' storage"; the scroll paddings and margins and
//! `overscroll-behavior` act only when something scrolls (ACID I11) — in
//! this frame they draw nothing.
//!
//! Derivation (the `ltr` column at x 0, the `rtl` one at x 30–57; each a
//! block flow, `box-sizing: border-box` where sizes are set):
//!
//! - Rows 0–1: 5 × 2 teal, `padding-inline-start` and `-block-start` 1 —
//!   `a` at (1, 1); under `rtl` at the column's right (x 53–57), the
//!   padding on the right, `a` right-aligned at x 56.
//! - Rows 2–4: 6 × 3, block sides `solid` (top) and `double` (bottom),
//!   inline-start red `solid`, inline-end blue `dashed thick`. `ltr`: left
//!   red `│`, right blue heavy `╏`; corners `┌` (top beats red at equal
//!   weight and style), `┒` (heavy blue), `╘` (double beats solid), `┛`
//!   (heavy beats double, its set); `dd` at x 1. `rtl` mirrors the inline
//!   sides: `┎` / `╏` / `┗` blue on the left, `┐` / `│` red / `╛` on the
//!   right; `dd` right-aligned, x 55–56.
//! - Rows 5–7: 4 × 3 `solid` with `start-start` and `end-end` radii 1:
//!   `ltr` `╭` top-left and `╯` bottom-right; `rtl` `╮` top-right and `╰`
//!   bottom-left.
//! - Row 9 (`margin-block-start: 1`): 2 wide, olive, `margin-inline-start`
//!   1, `padding-inline-end` 1, relatively moved by `inset-inline-start: 1`.
//!   `ltr`: placed at x 1, moved to x 2–3, `m` at 2. `rtl`: right margin 1
//!   — x 55–56 — moved left to x 54–55, the padding on the left, `m` at 55.
//! - Row 10: `efghij` in 4 cells with `overflow-inline: hidden`: `efgh`;
//!   under `rtl` (x 54–57) `ghij`. The `<br>`'s second line is clipped by
//!   `overflow-block: clip`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "27",
    spec: &[
        "CSS Logical 1 §2–§6; CSS 2.1 §9.4.3, §10.3.3; CSS Overflow 3 §2",
        "DIVERGENCES §1 horizontal-tb, border width, corner, rounded corner; §2 logical storage, border styles",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('o', "bg #808000"),
        ('r', "fg #ff0000"),
        ('b', "fg #0000ff"),
    ],
    grid: r#"
|                                                          |
|ttttt................................................ttttt|
| a                                                      a |
|ttttt................................................ttttt|
|┌────┒                                              ┎────┐|
|.....b..............................................b.....|
|│dd  ╏                                              ╏  dd│|
|r....b..............................................b....r|
|╘════┛                                              ┗════╛|
|.....b..............................................b.....|
|╭──┐                                                  ┌──╮|
|..........................................................|
|│  │                                                  │  │|
|..........................................................|
|└──╯                                                  ╰──┘|
|..........................................................|
|                                                          |
|..........................................................|
|  m                                                    m  |
|..oo..................................................oo..|
|efgh                                                  ghij|
|..........................................................|
"#,
};
