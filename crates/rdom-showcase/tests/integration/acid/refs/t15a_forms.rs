//! Tile 15a — form controls.
//!
//! Spec: HTML §4.10 (`placeholder`, `required`, `readonly`, `disabled`
//! through a `<fieldset>`, the number / date / time range limits — a
//! `time` range whose `max` is below its `min` wraps midnight, §4.10.5.1.9
//! "have a reversed range"), §4.16.3 (`:invalid`, `:read-only` /
//! `:read-write`, `:disabled`, `:in-range` / `:out-of-range`, `:default` —
//! a form's first submit button —, `:indeterminate` on a radio whose
//! group has no checked member, `:user-valid` / `:user-invalid` only after
//! interaction), §15.5 (the controls' rendering, left to the UA); CSS UI 4
//! §5 (outlines: the ring outside the border box, `outline-offset`
//! farther out or, negative, inside; `auto` the UA's focus ring), §6.3
//! (`accent-color`: checked marks and the progress bar, not the meter),
//! §7.1 (`appearance: none`), §7.2 (`field-sizing: content`); CSS
//! Pseudo-Elements 4 §4.3 (`::placeholder`). DIVERGENCES: §1 "A text field
//! has no UA border" (`padding: 0 1` on the `Field` background), "An
//! outline is a whole-cell ring", §2 "`accent-color` tints the glyphs a
//! control draws", "`appearance: none` strips rdom's chrome", "`field-sizing:
//! content` replaces the UA's fixed field size", "A resizable box is
//! resized from its corner cell" (`◢`, in the box's colour), the UA's
//! controls as `crates/rdom-style/src/ua/` writes them: buttons bold in the
//! accent (`AccentColor`, dodgerblue `#1e90ff`) between `[ ` and ` ]`, a
//! value-less submit `[ Submit ]`, the toggles' marks `[ ] ` / `[x] ` /
//! `[-] ` / `( ) ` / `(•) `, a closed drop-down's selected label in white on
//! the field with an accent `▾` one cell inside its right edge, a progress
//! bar and a meter of `█` / `░` (the meter's optimum zone in its UA
//! `limegreen`), the field background `#1f2123`.
//!
//! Derivation (six flex bands, items 1 apart unless noted; rows 0, 4, 6,
//! 8, 16, 21). A text field and a textarea are content-box (the UA lists
//! only the toggles, the buttons, `search`, `color`, `select`, `meter`
//! and `progress` as border-box, as the engines' sheets do), so a field's
//! `width: N` is N + 2 cells with its padding:
//!
//! - Row 0: a 10-wide field `abc`; a 10-wide field showing its
//!   placeholder `name` in the author's green; `field-sizing: content` on
//!   `ab` — its max-content plus padding, 4; a 10 × 3 textarea `hi` with
//!   its grip in its padding box's bottom-right cell (x 36, row 2); a
//!   10-wide (border-box) closed select showing `Two`, the selected
//!   option, and `▾` one cell inside its right edge (x 46) — on the
//!   select's field background, the `::after` being inside its padding
//!   box.
//! - Row 4: `form.ac` sets `accent-color` magenta: the checked box's, the
//!   indeterminate box's (its `indeterminate` attribute) and the checked
//!   radio's marks magenta, the unchecked ones in the text colour; the
//!   inline-blocks 4 wide with one collapsed space between. The progress
//!   bar, 8 wide at 1 / 4: 2 `█` and 6 `░`, magenta; the meter at 0.5 of
//!   0–1, 4 and 4, `limegreen` (the accent does not reach it).
//! - Row 6: `[ OK ]`, `[ Submit ]`, and under `appearance: none` the
//!   bracket-less `OK` (still bold and accent: the UA's colour and weight
//!   are its own declarations, not chrome) and a progress bar with no bar.
//! - Rows 8–14, `padding: 2`, items 4 apart from x 2: `outline: solid`
//!   around `ab` (x 2) — the ring at x 1–4, rows 9–11; `double` with
//!   `outline-offset: 1` around `cd` (x 8) — two cells out, x 6–11, rows
//!   8–12; a 4 × 3 box `1234` / `5678` / `90ab` (x 14) with
//!   `outline-offset: -1` — the ring on its own outer cells, over its
//!   text, leaving `67`; `outline: auto` around `gh` (x 22) — rounded, in
//!   the accent; a 3 × 3 `overflow: hidden` box (x 28) whose child `xy`
//!   (x 29, row 11) has a solid ring cut by the box at x 30; an inline
//!   `bb` in `aa bb cc` (x 35) — its ring replaces the spaces beside it
//!   and stands on the rows above and below the line.
//! - Rows 16–19: the `<fieldset disabled>` (UA `padding: 1`, 8 wide): its
//!   `L` in the text colour, with no weight — neither the legend nor the
//!   fieldset is a control the UA greys (ACID-FIX-6, ACID-FIX-8) — and its
//!   field, `:disabled`: the author's grey, the text in the UA's muted
//!   `GrayText` (`#7f868b`). Then the `required` empty field `:invalid`
//!   (dark red) — its `:user-invalid` rule does not apply, nothing was
//!   interacted with; the `readonly` field `:read-only` (navy); the
//!   `contenteditable` `e` `:read-write` (green); the numbers `9` of 1–5
//!   `:out-of-range` (olive) and `3` `:in-range` (teal — its `:user-valid`
//!   rule does not apply).
//! - Row 21: the form's first submit button `[ A ]` is `:default`
//!   (yellow), `[ B ]` keeps the accent; the two radios of group `q`, none
//!   checked, are `:indeterminate` — their marks inherit the orange; the
//!   date 2025-06-01 of 2026 `:out-of-range` (14 wide); the time 23:00 in
//!   the reversed range 22:00–02:00 `:in-range` (9 wide). Both are fields
//!   of the UA's (background, padding) that show no text: DIVERGENCES §2
//!   "Constraint validation on unrendered input types is partial" — rdom
//!   does not render the date and time types as fields (their value is
//!   the attribute's, not a text child's), so only the state colours show.
//!   (First derived with the values drawn, as a browser's widget shows
//!   them; changed for that entry.)

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "15a",
    spec: &[
        "HTML §4.10, §4.16.3, §15.5; CSS UI 4 §5–§7",
        "Selectors 4 §14; CSS Pseudo-Elements 4 §4.3",
        "DIVERGENCES §1 text field, outline; §2 accent-color, appearance, field-sizing, resizer",
    ],
    legend: &[
        ('f', "bg #1f2123"),
        ('p', "fg #00a000 bg #1f2123"),
        ('w', "fg #ffffff bg #1f2123"),
        ('a', "fg #1e90ff"),
        ('A', "fg #1e90ff bg #1f2123"),
        ('m', "fg #ff00ff"),
        ('l', "fg #32cd32"),
        ('B', "fg #1e90ff bold"),
        ('d', "bg #606060"),
        ('D', "fg #7f868b bg #606060"),
        ('r', "bg #800000"),
        ('n', "bg #000080"),
        ('g', "fg #00a000"),
        ('o', "bg #808000"),
        ('t', "bg #008080"),
        ('Y', "fg #ffff00 bold"),
        ('O', "fg #ffa500"),
    ],
    grid: r#"
| abc        name       ab   hi         Two    ▾           |
|ffffffffff.fppppfffff.ffff.ffffffffff.fwwwffffAf..........|
|                                                          |
|...........................ffffffffff.....................|
|                                    ◢                     |
|...........................ffffffffff.....................|
|                                                          |
|..........................................................|
|[x]  [ ]  [-]  (•)  ( )  ██░░░░░░ ████░░░░                |
|mmmm......mmmm.mmmm......mmmmmmmm.llllllll................|
|                                                          |
|..........................................................|
|[ OK ] [ Submit ] OK                                      |
|BBBBBB.BBBBBBBBBB.BB......................................|
|                                                          |
|..........................................................|
|      ╔════╗                                              |
|..........................................................|
| ┌──┐ ║    ║         ╭──╮            ┌──┐                 |
|.....................aaaa.................................|
| │ab│ ║ cd ║  ┌──┐   │gh│   ┌──    aa│bb│cc               |
|.....................a..a.................................|
| └──┘ ║    ║  │67│   ╰──╯   │xy      └──┘                 |
|.....................aaaa.................................|
|      ╚════╝  └──┘          └──                           |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                 r     e  9      3                        |
|.........rrrrrr.nnnnnn.g.oooooo.tttttt....................|
| L                                                        |
|..........................................................|
|  d                                                       |
|.dDdddd...................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|[ A ] [ B ] ( ) ( )                                       |
|YYYYY.BBBBB.OOOOOOOO.oooooooooooooo.ttttttttt.............|
"#,
};
