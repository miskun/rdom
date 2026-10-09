//! Tile 33 — states and the caret.
//!
//! Spec: Selectors 4 §9.4 (`:active`), §10 (`:focus`, `:focus-visible`,
//! `:focus-within` — the focused element and its ancestors), §14
//! (`:checked`, `:placeholder-shown`, `:enabled`, `:valid`, `:required`,
//! `:optional`), §11 / HTML §4.16.3 (`:open` on an open `details`); HTML
//! §6.6 (`focus()`); CSS UI 4 §6.2 (`caret` and its longhands:
//! `caret-shape: bar`, `caret-animation: manual`); CSS Pseudo-Elements 4
//! §4 / CSS Lists 3 (a list-item `::after` has a marker, `::after::marker`
//! styles it; `inside` puts `• ` before its text). DIVERGENCES §2
//! "`caret-shape` draws the painted caret in whole cells" (`bar` at a
//! line's end is `▏` in `caret-color`), "Caret paint composes
//! `caret-color` … and the rdom-extension `caret-text-color`" (the glyph
//! colour of a block caret — none here), the UA's
//! `input:focus-visible { background-color: #2d2f31 !important }` (a
//! focused field's background, over author rules), the UA's
//! `::placeholder` muted colour `#7f868b`, buttons bold, the toggles'
//! marks; the stage-2 script (I2, I3) presses and tabs for real.
//!
//! Derivation (one band, items 1 apart: x 0, 9, 15, 20, 27, 31, 37, 42,
//! 47, 52):
//!
//! - The wrapper (`padding: 0 1`) is `:focus-within`: maroon at x 0 and 7.
//!   The field (6 wide) is focused: the UA's focus background over all of
//!   it; `ab` yellow (`:focus-visible`) and underlined (`:focus`); the
//!   caret at the field's selection — `focus()` moves no selection (HTML
//!   §6.6.3) and a fresh field's is at its start — on `a`: a `bar` over a
//!   glyph is the underscore's underline, in `caret-color`, so `a`'s
//!   underline is red. (First derived after `b`, as a click at the end
//!   would put it, `▏` at x 4.)
//! - `[ A ]` red (`:active`), bold; `[x] ` green (`:checked`); the empty
//!   field with its placeholder `ph` on olive (`:placeholder-shown`);
//!   the open `details` green (`:open`): `▾ s`, then `x`; `[ E ]` green
//!   (`:enabled`); `v`'s field teal (`:valid`), `r`'s navy (`:required`),
//!   the empty one maroon (`:optional`).
//! - The `::after` list item: its marker `•` red, then `z`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "33",
    spec: &[
        "Selectors 4 §9.4, §10, §11, §14; HTML §4.16.3, §6.6",
        "CSS UI 4 §6.2; CSS Pseudo-Elements 4 §4; CSS Lists 3",
        "DIVERGENCES §2 caret-shape, caret paint",
    ],
    legend: &[
        ('m', "bg #800000"),
        ('f', "bg #2d2f31"),
        ('Y', "fg #ffff00 bg #2d2f31 underline"),
        ('U', "fg #ffff00 bg #2d2f31 ul #ff0000 underline"),
        ('R', "fg #ff0000 bold"),
        ('g', "fg #00a000"),
        ('G', "fg #00a000 bold"),
        ('o', "bg #808000"),
        ('p', "fg #7f868b bg #808000"),
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('r', "fg #ff0000"),
    ],
    grid: r#"
|  ab     [ A ] [x]   ph    ▾ s [ E ]  v    r        • z   |
|mfUYfffm.RRRRR.gggg.oppooo.ggg.GGGGG.tttt.nnnn.mmmm.r.....|
|                           x                              |
|...........................g..............................|
"#,
};
