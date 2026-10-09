//! Tile 12 — group opacity.
//!
//! Spec: CSS Color 4 §3.2 (`opacity` renders the element and its
//! descendants as a group, composited once at the given alpha; nested
//! groups multiply), Compositing 1 §5.1 (source-over: `α·src +
//! (1 − α)·dst`), CSS Color 4 §4.2 (8-bit channels); DIVERGENCES §2
//! "`opacity` composites per cell, one glyph per cell" and DESIGN "`opacity`
//! is group opacity" (a cell keeps one glyph: the layer's when `α ≥ 0.5`
//! or the backdrop has none, else the backdrop's, tinted toward the layer's
//! background, `α·layer_bg + (1 − α)·fg`; a background blends with the
//! backdrop's; `Color::Reset` blends as the dark canvas, black; `opacity:
//! 0` composites nothing).
//!
//! Derivation. The backdrop: grey `#646464` text `abc…z0…9` on row 0 (x
//! 0–35), and a 12 × 3 grey-bordered box on rows 2–4, all on the terminal
//! background (black when blended). The boxes, each absolutely positioned
//! on row 0 unless said:
//!
//! - `.t1`, 0.5, red `#c80000`, x 0–3, no text: background 0.5·200 =
//!   `#640000`; `abcd` stay, tinted 0.5·(200, 0, 0) + 0.5·(100, 100, 100) =
//!   `#963232`.
//! - `.g1` 0.5 holding `.g2` 0.5, green `#00c800`, x 6–9: 0.25 overall —
//!   background 0.25·200 = `#003200`; `ghij` seen through it, 0.25·(0, 200,
//!   0) + 0.75·(100, 100, 100) = `#4b7d4b`.
//! - `.t0`, `opacity: 0`, x 12–15: nothing; `mnop` as they were.
//! - `.t5`, 0.5, red, blue `#0000c8` text `ZZ`, x 18–21: at 0.5 the layer's
//!   glyphs win their cells, `Z` in 0.5·200 blue over the black backdrop
//!   background, `#000064`, on `#640000`; under its blank cells `uv` stay,
//!   tinted `#963232`.
//! - `.t4`, 0.4, red with blue `YY`, x 24–25: under 0.5 the backdrop's `yz`
//!   keep the cells, tinted 0.4·200 + 0.6·100 = 140 and 0.6·100 = 60 —
//!   `#8c3c3c` — on 0.4·200 = `#500000`.
//! - `.t6`, 0.5, red, x 0–5 of rows 2–3, over the box's border: the border
//!   glyphs stay, tinted `#963232`, on `#640000`; the box's empty inside
//!   takes the background alone.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "12",
    spec: &[
        "CSS Color 4 §3.2, §4.2",
        "Compositing 1 §5.1",
        "DIVERGENCES §2 opacity per cell; DESIGN group opacity",
    ],
    legend: &[
        ('t', "fg #963232 bg #640000"),
        ('d', "fg #646464"),
        ('n', "fg #4b7d4b bg #003200"),
        ('z', "fg #000064 bg #640000"),
        ('y', "fg #8c3c3c bg #500000"),
        ('r', "bg #640000"),
    ],
    grid: r#"
|abcdefghijklmnopqrZZuvwxyz0123456789  |
|ttttddnnnnddddddddzzttddyydddddddddd..|
|                                      |
|......................................|
|┌──────────┐                          |
|ttttttdddddd..........................|
|│          │                          |
|trrrrr.....d..........................|
|└──────────┘                          |
|dddddddddddd..........................|
"#,
};
