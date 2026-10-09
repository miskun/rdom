//! Tile 13 — overflow and scrollbars.
//!
//! Spec: CSS Overflow 3 §2 (`hidden` and `clip` clip at the padding edge;
//! `clip` per axis with the other `visible`; §3.2 `overflow-clip-margin`
//! moves the clip edge out), §2.2 (scrollable overflow — an absolutely
//! positioned descendant included), §3 (`scrollbar-gutter`: `stable`
//! reserves the gutter whether or not the box overflows, `both-edges` on
//! both sides; `auto` with classic scrollbars reserves it only while the
//! box overflows); CSS Overflow 4 §3 (`text-overflow: ellipsis` at the end
//! edge of the line — the left one in `rtl`), §4 (`line-clamp: N`: N lines
//! with a block ellipsis `…` after the last; the `-webkit-box` /
//! `-webkit-line-clamp` legacy pair as the standard form); CSSOM View
//! §4 (`scrollTop`); CSS Scrollbars 1 §2–§3 (`scrollbar-color` thumb /
//! track, `thin`); DIVERGENCES §1 "A scrollbar is one cell" (`thin`: no
//! track glyph, the thumb `│`; `scrollbar-color` fills the track's cells
//! with the track colour and draws the thumb on it), §2 "`::scrollbar` …
//! are rdom pseudo-elements" (`content` is the cell glyph); the
//! proportional thumb of the UA's scrollbar (Overflow 3 leaves the bar to
//! the UA; rdom's, `paint_pass::scrollbar`: size = track · viewport /
//! content, offset = scroll · (track − size) / (content − viewport) — the
//! numbers here divide exactly).
//!
//! Derivation (two flex bands; band 1 items at x 0, 9, 16, 23, 32, 39, 45,
//! 51):
//!
//! - x 0, `overflow: hidden`, 8 × 2: `aaaa` / `bbbbbbbbbbbb` cut to 8; the
//!   positioned teal child at x 6, 6 wide, cut to x 6–7.
//! - x 9, `overflow: auto`, 6 × 4, its load script scrolled 2 rows: the
//!   gutter is x 14, the scrollport x 9–13 shows `3`–`6`. Track 4 rows,
//!   content 8: a thumb of 4 · 4 / 8 = 2 rows at 2 · (4 − 2) / 4 = 1 —
//!   rows 1–2, `│` (thin) yellow on the navy track; rows 0 and 3 navy.
//! - x 16, 6 wide, `overflow-x: clip` with a 1-cell margin, `overflow-y:
//!   visible`: `cccccccccc` cut at x 16 + 6 + 1 = 23 (7 cells); `dd ee` and
//!   `ff` — the third line past the 2-row height shows.
//! - x 23, 8 wide, `nowrap` + `ellipsis`: `abcdefg…`; under `rtl` the line
//!   starts at the right and overflows the left (end) edge: its last 7
//!   cells `efghijk` with `…` at the end edge, x 23.
//! - x 32, 6 wide: `line-clamp: 2` over `aa bb` / `cc dd` / `ee ff`: two
//!   lines, `…` after the second; the `-webkit-box` pair with 3: `aa bb`,
//!   `cc dd`, `ee ff…` (`gg` gone).
//! - x 39, `overflow: auto`, 5 × 3: `x`, and a box positioned at row 8 —
//!   scrollable overflow to row 9, so it scrolls: gutter x 43, a thumb of
//!   3 · 3 / 9 = 1 row at 0, `█`, then the track `░`, both green
//!   (`::scrollbar` / `::scrollbar-thumb`).
//! - x 45, `scrollbar-gutter: stable`, 5 wide: the gutter (x 49) is kept
//!   with nothing to scroll, so `ab cd` wraps in 4: `ab` / `cd`.
//! - x 51, `stable both-edges`: gutters at x 51 and 55, `ab` / `cd` from
//!   x 52.
//! - Row 6, x 0: `.ou` (`overflow: auto`, 8 × 3) holds `12345678` and
//!   `.in`, a 2-row scroller of four lines. The inner box's overflow is its
//!   own: `.ou`'s content is 3 rows and fits, so it has no scrollbar and —
//!   `scrollbar-gutter: auto` — no gutter, and `12345678` fills its 8 cells.
//!   `.in` shows `a`, `b` with its bar at x 7: a 1-row thumb (2 · 2 / 4) at
//!   0.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "13",
    spec: &[
        "CSS Overflow 3 §2, §3; Overflow 4 §3, §4",
        "CSS Scrollbars 1 §2–§3; CSSOM View §4",
        "DIVERGENCES §1 scrollbar one cell; §2 ::scrollbar",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('Y', "fg #ffff00 bg #000080"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|aaaa     3      cccccccabcdefg… aa bb  x   █ ab     ab    |
|......tt......n............................g..............|
|bbbbbbbb 4    │ dd ee  …efghijk cc dd…     ░ cd     cd    |
|..............Y............................g..............|
|         5    │ ff              aa bb      ░              |
|..............Y............................g..............|
|         6                      cc dd                     |
|..............n...........................................|
|                                ee ff…                    |
|..........................................................|
|                                                          |
|..........................................................|
|12345678                                                  |
|..........................................................|
|a      █                                                  |
|.......g..................................................|
|b      ░                                                  |
|.......g..................................................|
"#,
};
