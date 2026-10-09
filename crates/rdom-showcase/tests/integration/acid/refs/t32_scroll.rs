//! Tile 32 — scrolling at rest.
//!
//! Spec: CSS Scroll Snap 1 §5 (`scroll-snap-type: y mandatory` — the
//! container rests at a snap position after any scroll, a programmatic
//! one included), §4 (`scroll-padding` insets the snapport: an item's
//! `start` snap position is its top less the padding; `scroll-margin`
//! outsets the snap area — 0 here), §6.2 (`scroll-snap-stop` acts on
//! directional scrolls only); CSSOM View §4 (`scrollTop =`, its
//! `scroll-behavior` `auto` instant); CSS Overscroll Behavior 1 (acts on
//! scroll chaining only — none here); CSS Overflow 3 §3 (both scrollbars,
//! the corner left blank). DIVERGENCES §2 "Scroll snapping decides per
//! scroll operation" (`scrollTop =` rests at the snap position nearest its
//! destination), §2 "`::scrollbar` … are rdom pseudo-elements" (`content`
//! the cell glyph on both axes, `:vertical` / `:horizontal` the thumb of
//! one axis), §1 "A scrollbar is one cell" and the UA's thumb `track ·
//! viewport / content` (rounded down, at least 1).
//!
//! Derivation:
//!
//! - `.sn` (x 0, 8 × 4, no bar): five 3-row items (teal, navy, …) in a
//!   3-row snapport (4 less the padding) — none longer than it, so each
//!   has one snap position, 3k − 1 clamped: 0, 2, 5, 8, 11 (11 the end).
//!   The script's `scrollTop = 4` rests at the nearest, 5: row 0 is the
//!   last row of `i1` (navy), rows 1–3 `i2` (teal, `i2` on row 1) — one
//!   row below the snapport's top, its `scroll-padding`. (First derived in
//!   a 3-row box, whose 2-row snapport the 3-row items are longer than:
//!   §6.2.3 makes every offset at which such an area covers the snapport
//!   valid, and 4 lies as near to item 1's covering offsets as to 5 — the
//!   box is a row taller now, so the snap is unambiguous.)
//! - `.bars` (x 10, 6 × 3, six lines of `0123456789`): both bars — the
//!   vertical at x 15 over rows 0–1, the horizontal on row 2 over x 10–14,
//!   the corner blank; the view 5 × 2, `01234` twice. Thumbs: vertical
//!   2 · 2 / 6 → 1 at the top, `█` yellow, then the track `░` green;
//!   horizontal 5 · 5 / 10 → 2, `▬▬` yellow, then `░░░`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "32",
    spec: &[
        "CSS Scroll Snap 1 §4–§6; CSSOM View §4; CSS Overscroll Behavior 1; CSS Overflow 3 §3",
        "DIVERGENCES §2 scroll snapping, ::scrollbar; §1 scrollbar one cell",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('y', "fg #ffff00"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|          01234█                                          |
|nnnnnnnn.......y..........................................|
|i2        01234░                                          |
|tttttttt.......g..........................................|
|          ▬▬░░░                                           |
|tttttttt..yyggg...........................................|
|                                                          |
|tttttttt..................................................|
"#,
};
