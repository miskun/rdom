//! Tile 29 — text and font longhands.
//!
//! Spec: CSS Fonts 4 §2–§3 (`font-size`, `font-family`, `font-stretch` /
//! `font-width`, `font-variant`); CSS Text Decoration 4 §2–§4 (the
//! `text-decoration-line` / `-style` / `-color` longhands, `wavy` the
//! curly underline; `-thickness`, `text-underline-offset` / `-position`,
//! `text-decoration-skip-ink`); CSS Text 3 §9.2 (`word-spacing` adds to
//! each word separator), §3 / Text 4 (`white-space-collapse: preserve`
//! keeps the two spaces, `text-wrap-mode: nowrap`), §5.5 (`word-wrap`,
//! the alias of `overflow-wrap`: `break-word` breaks the long word at the
//! box's 4 cells), §5.3 (`line-break: anywhere` breaks between any two
//! letters), §5.4 (`hyphens: auto` — a soft hyphen breaks and shows `-`),
//! §6 (`text-align-all: right`; `text-justify` acts only on justified
//! lines); CSS Text 4 (`text-wrap-style: pretty` on a one-line block
//! changes nothing); CSS Overflow 4 §4 (`max-lines: 1` with `continue:
//! discard` clamps after the first line, `block-ellipsis: auto` puts `…`
//! after it); CSS Lists 3 (`list-style: square inside` — the marker `▪ `
//! inside the item after the list's padding; `list-style-image: none`;
//! `marker-side`); CSS UI 4 §4.2 (`resize` on a scroll container shows the
//! resizer), §7.1 (`-webkit-appearance: none` strips a button's chrome);
//! `pointer-events` and `cursor` act on the pointer only. DIVERGENCES §1
//! "CSS with no meaning on a character grid" (the font properties parse
//! and draw nothing), §1 sub-cell geometry (thickness, offset, position,
//! skip-ink kept, drawing nothing), §2 "Text decorations are the
//! terminal's SGR attributes" (`wavy` is the curly underline, its colour
//! SGR 58), §2 "Line breaking is a UAX #14 subset" (`hyphens: auto` is
//! `manual`), §1 "Letter and word spacing are whole blank cells", §2 "A
//! resizable box is resized from its corner cell" (`◢`), §2
//! "`appearance: none` strips rdom's chrome" (`-webkit-appearance` its
//! alias).
//!
//! Derivation (band 1 at row 0, items 2 apart: x 0, 6, 12, 23, 29, 35, 40,
//! 45; band 2 at row 3: x 0, 8, 17, 22):
//!
//! - `font` as text; `deco` with a red curly underline; `a b c` with two
//!   cells more at each separator: `a`, `b`, `c` at 12, 16, 20; `x  y`
//!   with both spaces; `abcd` / `efgh`; `abc` / `def`; `ab-` / `cd`; `ab
//!   cd` right-aligned in 7 — from x 47.
//! - `aa bb cc` in 6, clamped after `aa bb`: `aa bb…`. The `inside` square
//!   marker after the list's four cells of padding: `▪` at 12, `x` at 14.
//!   The 3 × 2 resizable box's grip at its bottom-right cell (19, 4). The
//!   button without its brackets: `B`, bold in the accent.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "29",
    spec: &[
        "CSS Fonts 4 §2–§3; CSS Text 3 §3, §5, §6, §9; CSS Text Decoration 4 §2–§4",
        "CSS Overflow 4 §4; CSS Lists 3; CSS UI 4 §4.2, §7.1",
        "DIVERGENCES §1 N/A fonts, sub-cell, spacing; §2 decorations, breaking, resizer, appearance",
    ],
    legend: &[('d', "curly ul #ff0000"), ('B', "fg #1e90ff bold")],
    grid: r#"
|font  deco  a   b   c  x  y  abcd  abc  ab-    ab cd      |
|......dddd................................................|
|                             efgh  def  cd                |
|..........................................................|
|                                                          |
|..........................................................|
|aa bb…      ▪ x       B                                   |
|......................B...................................|
|                   ◢                                      |
|..........................................................|
"#,
};
