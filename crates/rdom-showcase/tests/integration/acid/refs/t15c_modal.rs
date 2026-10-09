//! Tile 15c — a modal dialog and its `::backdrop`.
//!
//! Spec: HTML §4.11.4 (`showModal()`: the dialog in the top layer, modal),
//! the rendering section (`dialog:modal { position: fixed; inset-block: 0;
//! margin: auto }`, fit-content, `dialog { background-color: Canvas;
//! color: CanvasText }`), CSS Position 4 §3 (the top layer above every
//! stacking context — a `z-index: 5` box included — and outside every
//! clip; `::backdrop` right below its element, over the viewport), CSS
//! Color 4 §4.2 / Compositing 1 §5.1 (a translucent background composites
//! source-over, `α·src + (1 − α)·dst`, in 8-bit channels). DIVERGENCES §2
//! "Translucent colors composite per cell, one glyph per cell" (a
//! translucent background blends with the one beneath and tints the
//! glyphs it leaves, `α·bg + (1 − α)·fg`; `Color::Reset` blends as the
//! dark canvas — black, its text white), "Only a modal `<dialog>` leaves
//! the flow, and its `::backdrop` is unstyled by default" (centred, its
//! size capped at the viewport less two cells), "The system colors are
//! the terminal's"; the UA dialog (`padding: 1 2`, a rounded single
//! border in the accent `#1e90ff`).
//!
//! Derivation (the whole page is the tile, at row 1; tile rows are page
//! rows − 1):
//!
//! - The dialog, `Modal` (5 wide): 5 + 4 + 2 = 11 wide, 1 + 2 + 2 = 5
//!   tall, centred in the 120 × 50 viewport — x (120 − 11) / 2 = 54, page
//!   row (50 − 5) / 2 = 22, tile rows 21–25: `╭─────────╮`, its sides, and
//!   `Modal` at x 57, tile row 23; every other cell inside blank on the
//!   default background (`Canvas`), no letter of the page text through
//!   it. Its in-flow parent's 1 × 1 clip does not reach it.
//! - The backdrop, `rgb(0 0 100 / 60%)` (α = 153 / 255 = 0.6), over every
//!   other cell: an empty cell's black canvas becomes 0.6 · (0, 0, 100) =
//!   `#00003c`; the page text (tile rows 18–27, `abcdefghij` × 12, white
//!   on black when blended) keeps its letters, tinted 0.6 · (0, 0, 100) +
//!   0.4 · (255, 255, 255) = `#6666a2`, on `#00003c`; the `z-index: 5`
//!   box (x 50–69, tile rows 21–23, green `#006400`, above the text, so
//!   its cells hold no letter) becomes 0.6 · (0, 0, 100) + 0.4 · (0, 100,
//!   0) = `#00283c`, except where the dialog covers it.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "15c",
    spec: &[
        "HTML §4.11.4, rendering dialog:modal; CSS Position 4 §3",
        "CSS Color 4 §4.2; Compositing 1 §5.1",
        "DIVERGENCES §2 translucent colors, modal dialog, system colors",
    ],
    legend: &[
        ('k', "bg #00003c"),
        ('t', "fg #6666a2 bg #00003c"),
        ('z', "bg #00283c"),
        ('a', "fg #1e90ff"),
    ],
    grid: r#"
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij    ╭─────────╮     abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|ttttttttttttttttttttttttttttttttttttttttttttttttttzzzzaaaaaaaaaaazzzzztttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij    │         │     abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|ttttttttttttttttttttttttttttttttttttttttttttttttttzzzza.........azzzzztttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij    │  Modal  │     abcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|ttttttttttttttttttttttttttttttttttttttttttttttttttzzzza.........azzzzztttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcd│         │fghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttta.........attttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcd╰─────────╯fghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|ttttttttttttttttttttttttttttttttttttttttttttttttttttttaaaaaaaaaaattttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij|
|tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
|                                                                                                                        |
|kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk|
"#,
};
