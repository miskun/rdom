//! Tile 9b — lists and markers.
//!
//! Spec: CSS Lists 3 §3.1 (`::marker` text: the counter in the list's
//! style plus its suffix), §3.5 (an `outside` marker hangs before the
//! item's first line box, its end against the content edge; `inside` puts
//! it at the start of the first line), §4.6 (`list-item` counts the items;
//! `display: inline list-item` keeps its marker inside its line); CSS
//! Counter Styles 3 §6.1–§6.2 (`decimal` and `lower-alpha` / `upper-roman`
//! with the suffix `. `; `disc` `•`, `circle` `◦`, `square` `▪` with the
//! suffix ` `); HTML §4.4.5 (`start`, `reversed`: counting down from the
//! item count), §4.4.8 (`value`), §15.3.8 (`ul` / `ol` 40px of inline-start
//! padding — four cells here, DIVERGENCES via the UA comment; `type="I"` /
//! `type="a"`; nested `ul`s `circle`, then `square`); CSS Pseudo-Elements 4
//! §4 (a `::before` with `display: list-item` has its own `::marker`,
//! `::before::marker`), §3.2 (`::marker` takes `color` and `content`);
//! DIVERGENCES §2 "An outside list marker is drawn beside its item's first
//! line" (in whole cells; a marker wider than the padding overflows the
//! list's box and is cut by what clips it, C10G-MARKER-CLIP; an empty item's
//! marker makes a line of its own; a right-hanging marker is written in
//! visual order).
//!
//! Derivation (two flex bands, items a cell apart; every list has four
//! cells of padding at its inline start):
//!
//! - x 0, the tile's left edge: `<ol start=9>` — `9. ` (3 cells) ends at
//!   the content edge, x 1–3; `10. ` fills x 0–3. `<ol type=I start=3>` —
//!   `III. ` is 5 cells, x −1–3: the tile clips its first cell, `II. `
//!   shows (as the screen edge would). With `margin-left: 2` the same list's
//!   content starts at x 6 and `III. ` is whole at x 1.
//! - x 15: an item whose `p` wraps in 8 cells — `1. ` hangs at x 16, the
//!   text at x 19 and its second line under the text, not the marker; an
//!   empty item still shows its marker on a line of its own (`2.`); an
//!   `inside` item starts its line with `3. ` at the content edge.
//! - x 28: three nested `ul`s whose first lines are one line: the markers
//!   `•`, `◦`, `▪` each hang in its list's padding (x 30, 34, 38) beside
//!   `x` at x 40. Below, an `rtl` `ul`: its padding is on the right, so
//!   the content is x 28–37 with `ab` at its start, the right (x 36), and
//!   the marker hangs to the right, from x 38, written in visual order —
//!   `" •"`, the bullet at x 39 — as DIVERGENCES §2 "An outside list marker
//!   is drawn beside its item's first line" says for a right-hanging
//!   marker, and as a browser's bidi shows the right-to-left run `• `.
//!   (Corrected while building: the first derivation applied §1's general
//!   "no bidirectional reordering" — logical `• ` — where the marker entry,
//!   the specific rule, writes it reversed.)
//! - x 43: `<ol reversed>` of two — `2.` then `1.`; `<li value=7>` then
//!   the next item `8.`; `<ol type=a>` — `a.`.
//! - Row 6, x 0: `::marker { color }` reddens `•` but not `red`; the
//!   `::marker { content: "→ " }` item shows `→ ` in the same red.
//! - x 11 (`padding-left: 4`): a `::before` with `display: list-item` is a
//!   block line `B` with its own `square` marker `▪ ` hanging before it,
//!   green from `::before::marker`; `body` follows on the next row.
//! - x 22: an `inline list-item` span's marker `• ` sits in the line, at
//!   the span's start: `x • yy`.
//!
//! The `::marker:hover` rules do not apply: nothing is hovered.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "9b",
    spec: &[
        "CSS Lists 3 §3.1, §3.5, §4.6",
        "CSS Counter Styles 3 §6.1–§6.2",
        "HTML §4.4.5, §4.4.8, §15.3.8",
        "CSS Pseudo-Elements 4 §3.2, §4",
        "DIVERGENCES §2 outside list marker (rtl in visual order); C10G-MARKER-CLIP",
    ],
    legend: &[('r', "fg #c00000"), ('g', "fg #00a000")],
    grid: r#"
| 9. a           1. aa bb cc   •   ◦   ▪ x   2. a          |
|..........................................................|
|10. b              dd               ab •    1. b          |
|..........................................................|
|II. c           2.                          7. b          |
|..........................................................|
| III. c            3. dd ee                 8. c          |
|..........................................................|
|                                            a. x          |
|..........................................................|
|                                                          |
|..........................................................|
|  • red      ▪ B      x • yy                              |
|..r..........g............................................|
|  → txt        body                                       |
|..r.......................................................|
"#,
};
