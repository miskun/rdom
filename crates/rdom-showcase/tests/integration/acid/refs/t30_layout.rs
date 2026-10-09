//! Tile 30 — layout longhands.
//!
//! Spec: CSS Flexbox 1 §5.3 (`flex-flow`), §7.3 (the `flex` longhands),
//! §9.7 (resolving flexible lengths: free space 10 − 5 to the growing
//! item), §8.4 / §9.4 (a wrapping container is multi-line, so
//! `align-content: flex-end` moves its one line to the bottom); CSS Grid 2
//! §7.4 (`grid: <rows> / <columns>`), §7.6 (`grid-auto-columns` sizes an
//! implicit column), §8.3 (the line longhands), §8.5 (auto-placement: the
//! definite items first, then one with a definite column, then the rest in
//! the first free cell); CSS Box Alignment 3 §6 (`justify-items`,
//! `place-self` overriding it, `place-items`, `place-content`); CSS
//! Multi-column 1 §3, §4 (`column-count`, the `column-rule` longhands);
//! CSS Fragmentation 3 §3 (`break-after` / `-inside`, and `page-break-*`
//! as their legacy aliases — `auto` and `avoid` on one-line blocks change
//! no break); CSS UI 4 §5 (the `outline` longhands); CSS Anchor Positioning
//! 1 §4 (`position-try`, `position-try-order` — the base position fits,
//! so it is kept); CSS Position 4 (`overlay` acts only in a transition).
//! DIVERGENCES §1 "Alignment free space is shared in whole cells"
//! (`center` rounds down), "Grid tracks are whole cells", "An outline is
//! a whole-cell ring" (`thick` heavy), §2 "Multi-column layout is laid out
//! in whole cells" (the rule in the gap).
//!
//! Derivation (one band, items 2 apart: x 0, 12, 23, 28, 37, 42):
//!
//! - `.fl` (10 × 3): `a` grows from 2 to 7 (teal), `b` keeps 3 (navy); the
//!   line at the bottom, row 2.
//! - `.gr`: columns 2, 2, 2 and an implicit 3 (x 12, 14, 16, 18), two
//!   rows of 1. `p` (rows 2–3, columns 2–4) aligned to its area's end — x 17,
//!   row 1 (olive); `q` (column 4) on row 0 at its end, x 20 (maroon);
//!   `r` (`place-self: start`, both lines auto) after the sparse cursor,
//!   which `q`'s definite column left at column 4: past the last column it
//!   wraps to row 2, column 1 — x 12, row 1 (teal). (First derived in row
//!   0's first free cell, as `dense` would; §8.5 step 4's cursor only moves
//!   forward.)
//! - `.pi` (3 × 2): `c` at the end row and the centre column — (24, 1).
//! - `.cc` (7 wide, two columns of 3): `d1`, `d2` | `d3`, `d4`, the red
//!   rule at x 31.
//! - `.ol` inside a padding of 1: its heavy blue ring around `o`.
//! - `.pt` under its inline anchor `an`: (42, 1).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "30",
    spec: &[
        "CSS Flexbox 1 §5.3, §7.3, §8.4, §9.7; CSS Grid 2 §7.4, §7.6, §8.3, §8.5",
        "CSS Box Alignment 3 §6; Multi-column 1 §3–§4; Fragmentation 3 §3; CSS UI 4 §5; Anchor Positioning 1 §4",
        "DIVERGENCES §1 alignment, grid tracks, outline; §2 multi-column",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('r', "fg #ff0000"),
        ('b', "fg #0000ff"),
    ],
    grid: r#"
|                    q       d1 │d3   ┏━┓  an              |
|....................m..........r.....bbb..................|
|            r    p      c   d2 │d4   ┃o┃  pt              |
|............t....o.............r.....b.b..................|
|a      b                             ┗━┛                  |
|tttttttnnn...........................bbb..................|
"#,
};
