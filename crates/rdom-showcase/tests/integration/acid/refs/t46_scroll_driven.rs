//! Tile 46 — scroll-driven animations, at rest (the state step I18 starts
//! from).
//!
//! Spec: Scroll-driven Animations 1 §2.1 (`scroll()`: the nearest scroll
//! container's block axis, 0% at its scroll origin — the teal bar's
//! `width` 0 at offset 0), §2.2 / §4.2 (a named timeline seen outside its
//! scroller through `timeline-scope`: the olive bar at the `rtl`
//! scroller's origin, its right edge — 0%), §3.1 (the view items, below
//! the scrollport, out of sight); CSS Overflow 3 §3 (`scrollbar-width:
//! none`: no bar, no gutter); the tile's `p { margin: 0 }`.
//!
//! Derivation: the scroller's rows 0–3 — the bar's row (width 0: blank),
//! `r1`, `r2`, `r3`; row 5 the `rtl` scroller (empty); row 6 the olive
//! bar at width 0 (blank).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "46",
    spec: &["Scroll-driven Animations 1 §2.1, §2.2, §3.1, §4.2; CSS Overflow 3 §3"],
    legend: &[],
    grid: r#"
|                                      |
|......................................|
|r1                                    |
|......................................|
|r2                                    |
|......................................|
|r3                                    |
|......................................|
|                                      |
|......................................|
|                                      |
|......................................|
|                                      |
|......................................|
"#,
};
