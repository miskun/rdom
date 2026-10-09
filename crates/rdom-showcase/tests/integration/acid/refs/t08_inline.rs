//! Tile 8 — inline formatting.
//!
//! Spec: CSS Text 3 §4 (white space processing; §4.1.3 a collapsible
//! space at a line's end is removed, a preserved one under `pre-wrap`
//! hangs, under `break-spaces` it does not and may wrap), §5 (line
//! breaking; §5.2 `word-break: break-all` / `keep-all`, §5.5
//! `overflow-wrap: anywhere`, §5.4 a soft hyphen breaks and shows a
//! hyphen), §2.1 (`text-transform`), §4.2 (`tab-size`: tab stops every 4
//! cells), §8.1 (`text-indent`, `hanging` indenting all lines but the
//! first), §7.1 (`text-align`), §6.4 (`justify`), §7.2 (`text-align-last`);
//! CSS Text 4 (`text-wrap: balance`); CSS 2.1 §10.8 (line boxes; an inline
//! block's baseline is its last line box's; `vertical-align` `sub` /
//! `super` / `middle` / `top` / `bottom`), CSS Inline 3 §5.1 (`line-height`
//! and half-leading), CSS Values 4 §6.1.1 (`lh`); CSS Text Decoration 4 §2
//! (decorations propagate to in-flow descendants' text, not into an
//! atomic inline), §2.2–§2.3 (line styles and colour); CSS Fonts 4 §2.2
//! (`600`, `bolder`, `lighter` by the inherited weight), §2.4 (`oblique`),
//! §3.7 (the `font` shorthand resets `line-height` to `normal`); DIVERGENCES
//! §2 "`line-height` is whole rows", "`vertical-align` is whole rows",
//! "Text decorations are the terminal's SGR attributes", "The font is drawn
//! as SGR bold and italic", "A line box holds its inline blocks in whole
//! rows", "Justification is whole cells", "`text-wrap-style` choices",
//! "Line breaking is a UAX #14 subset", §1 "Alignment free space" (centre
//! rounds down).
//!
//! Derivation. Five flex bands, a row apart, their blocks a cell apart
//! (`align-items: flex-start`: each block at its band's top row).
//!
//! Band 1, rows 0–2. Six 5-cell clipping blocks hold `aa␣␣bb⏎cc dd`:
//! - `normal` (x 0): spaces and the newline collapse — `aa bb` / `cc dd`.
//! - `nowrap` (x 6): one line `aa bb cc dd`, clipped to `aa bb`.
//! - `pre` (x 12): `aa  bb` clipped to `aa  b`, then `cc dd`.
//! - `pre-wrap` (x 18): `aa␣␣bb` is 6 > 5: the break follows the spaces,
//!   which hang: `aa` / `bb` / `cc dd`.
//! - `pre-line` (x 24): spaces collapse, the newline stays: `aa bb` /
//!   `cc dd`.
//! - `break-spaces` (x 30): `aa␣␣` fits (4 cells, the spaces kept), `bb`
//!   wraps: `aa` / `bb` / `cc dd` (the spaces are blank cells).
//! - The atom block (x 36): `x`, an inline block (`ib`, a cell of padding
//!   each side, a border: 6 × 3), `y`. The line holds the atom whole; its
//!   baseline is its content row, so `x` and `y` sit on row 1 beside it.
//! - `sub` / `super` (x 46): `a₂b³c` — the `sub` a row below the baseline,
//!   the `sup` a row above; the line grows to 3 rows, baseline row 1.
//!
//! Band 2, rows 4–6:
//! - x 0, 10 wide: `ab cd ef gh ij` breaks after `ef` (the next word would
//!   make 11). The span `cd ef gh` is navy, bold, italic, underlined across
//!   the wrap: `cd ef` on row 4 (its inner space too), `gh` on row 5; the
//!   space at the break is removed, and the space after the span is not
//!   the span's.
//! - x 11, 4 wide, `overflow-wrap: anywhere`: `abcdefghij` → `abcd` /
//!   `efgh` / `ij`.
//! - x 16, 4 wide, `word-break: break-all`: `ab cdefgh` → `ab c` / `defg` /
//!   `h`.
//! - x 21, 4 wide, `keep-all`, clipped: `日本語の文` has no break, so one line,
//!   `日本` showing.
//! - x 26, 4 wide: `ab­cdef` (a soft hyphen) breaks there: `ab-` / `cdef`.
//! - x 31, `pre`, `tab-size: 4`: `a⇥b` → `b` at the stop 4; `ab⇥c` → `c`
//!   at 4.
//! - x 40: `AB` (`uppercase`), `Cd Ef` (`capitalize`), `gh` (`lowercase`).
//!
//! Band 3, rows 8–9:
//! - x 0, 8 wide, `text-indent: 2`: `aa bb` from x 2 (6 cells left), then
//!   `cc dd`.
//! - x 9, `2 hanging`: the first line unindented, `aa bb cc` (8); `dd` at
//!   x 9 + 2.
//! - x 18, 7 wide, `center`: `ab` with (7 − 2) / 2 = 2.5 → 2 leading.
//! - x 26, 5 wide, `right`: `ab` at x 29.
//! - x 32, 9 wide, `justify` then `text-align-last: right`: `aa bb cc` has
//!   one free cell and two spaces: the first gets it, `aa  bb cc`; the last
//!   line `dd` right: x 39.
//! - x 42, 12 wide, `balance`: greedy is `aaa bbb ccc` / `ddd`; the
//!   narrowest width keeping two lines is 7: `aaa bbb` / `ccc ddd`.
//!
//! Band 4, rows 11–14:
//! - x 0, `line-height: 3`, navy: 3 rows; half-leading ⌊(3 − 1) / 2⌋ = 1
//!   above, so `x` on row 12.
//! - x 6, `line-height: 2; height: 2lh`, teal: 4 rows; `y` on row 11 (no
//!   leading above, one below).
//! - x 10, `line-height: 200%`, purple: 2 rows; `z` on row 11.
//! - x 14: `.u` is a curly (`wavy`) red underline: `ab` and `gh` curly,
//!   underline colour red; `cd` (`.o`, an overline of its own) carries the
//!   propagated underline and its overline; `ef` is an inline block, which
//!   decorations do not enter: plain.
//! - x 35: `600` bold; `bolder` from 400 → 700, bold; `lighter` inside `b`
//!   (700) → 400, plain; `obl` (`oblique 10deg`) italic. Below it `.fs`:
//!   `line-height: 3` then `font: bold 16px serif`, which resets
//!   `line-height` to `normal` — one navy row, `x` bold.
//!
//! Band 5, rows 16–18:
//! - x 0: `a`, a 3-row inline block `1 2 3` (`middle`: its middle row on
//!   the baseline), `b`, a 2-row `4 5` (`top`), `c`, a 2-row `6 7`
//!   (`bottom`), `d`. The middle box spans a row above and below the
//!   baseline, so the line is 3 rows with its baseline on row 17; the top
//!   box takes rows 16–17, the bottom one 17–18.
//! - x 8: `dbl` double underline, `dsh` dashed, `dot` dotted, `str`
//!   (`<s>`) struck through, `ovl` overlined — in the text colour.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "8",
    spec: &[
        "CSS Text 3 §2.1, §4, §5, §6.4, §7, §8.1; Text 4 text-wrap",
        "CSS 2.1 §10.8; CSS Inline 3 §5.1",
        "CSS Text Decoration 4 §2",
        "CSS Fonts 4 §2.2, §2.4, §3.7",
        "DIVERGENCES §2 line-height, vertical-align, decorations, font, justification",
    ],
    legend: &[
        ('h', "bg #000080 bold italic underline"),
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('p', "bg #800080"),
        ('c', "curly ul #c00000"),
        ('C', "curly ul #c00000 overline"),
        ('B', "bold"),
        ('I', "italic"),
        ('N', "bg #000080 bold"),
        ('D', "double"),
        ('S', "dashed"),
        ('T', "dotted"),
        ('X', "strike"),
        ('O', "overline"),
    ],
    grid: r#"
|aa bb aa bb aa  b aa    aa bb aa     ┌────┐      3        |
|..........................................................|
|cc dd       cc dd bb    cc dd bb    x│ ib │y  a b c       |
|..........................................................|
|                  cc dd       cc dd  └────┘    2          |
|..........................................................|
|                                                          |
|..........................................................|
|ab cd ef   abcd ab c 日本 ab-  a   b    AB Cd Ef gh       |
|...hhhhh..................................................|
|gh ij      efgh defg      cdef ab  c                      |
|hh........................................................|
|           ij   h                                         |
|..........................................................|
|                                                          |
|..........................................................|
|  aa bb  aa bb cc   ab       ab aa  bb cc aaa bbb         |
|..........................................................|
|cc dd      dd                          dd ccc ddd         |
|..........................................................|
|                                                          |
|..........................................................|
|      y   z   abcdefgh             600 bolder lighter obl |
|nnnnn.ttt.ppp.ccCC..cc.............BBB.BBBBBB.........III.|
|x                                  x                      |
|nnnnn.ttt.ppp......................Nnnnnnnnnnnnnnnnnnnnnnn|
|                                                          |
|nnnnn.ttt.................................................|
|                                                          |
|......ttt.................................................|
|                                                          |
|..........................................................|
| 1 4    dbl dsh dot str ovl                               |
|.n.t....DDD.SSS.TTT.XXX.OOO...............................|
|a2b5c6d                                                   |
|.n.t.p....................................................|
| 3   7                                                    |
|.n...p....................................................|
|                                                          |
|..........................................................|
"#,
};
