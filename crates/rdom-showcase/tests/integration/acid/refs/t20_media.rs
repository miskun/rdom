//! Tile 20 — media queries.
//!
//! Spec: Media Queries 4 §2 (a comma list matches when any query does;
//! `not` negates a query; `only` hides it from legacy UAs and changes
//! nothing; `print` does not match a screen), §3 (`and` / `or`; Kleene
//! logic: an unknown feature or value is "unknown", and so is its `not`,
//! and a query that is unknown at the top is false), §4 (range syntax,
//! `min-` / `max-`), §1.3 (`em` in a query is the initial font size,
//! 16px), Media Queries 5 §12.5 (`prefers-color-scheme`); CSS Color 5
//! `light-dark()`; CSS Conditional 3 §2 (`@media` inside `@layer` and, by
//! CSS Nesting 1, inside a style rule); HTML §4.2.6 (a `<style media>`
//! sheet applies while its media matches); CSS Cascade 5 §3 (an `@import`
//! with a media list imports conditionally). DIVERGENCES §2 "Media
//! features answer for a terminal" (the viewport 120 × 50 columns × rows;
//! a pixel value selects at 8px a column, so `600px` is 75 columns and
//! `15em` 30; `ex` has no cell measure — unknown; `screen`, `grid` 1,
//! `color` 8, `hover: hover`, `pointer: fine`; the scheme dark for a
//! document no terminal answered, the headless `App`'s), "The preferred
//! color scheme is the terminal's" (`light-dark()` takes its dark arm).
//!
//! Derivation: the words wrap at 58 (spaces between); green where the
//! query holds at 120 × 50 — `range` (40 ≤ 120 < 130), `minw`, `and`
//! (landscape: 120 > 50), `list` and `or` (50 > 40), `notp`, `only`, `px`
//! (120 ≥ 75), `dark`, `ld` (the dark arm green), `root` (the `:root`
//! variable the dark `@media` set), `hov`, `ptr`, `grd`, `col` (8 ≥ 8),
//! `lay`, `nest`, `smed`, `imp`; the rest default — `maxw`, `print`,
//! `unk` and `nunk` (unknown either way), `npx`, `nar` (120 > 30), `ex`
//! (unknown), `sprn`, `impp`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "20",
    spec: &[
        "Media Queries 4 §1.3, §2–§4; Media Queries 5 §12.5",
        "CSS Color 5 light-dark(); CSS Conditional 3 §2; HTML §4.2.6; CSS Cascade 5 §3",
        "DIVERGENCES §2 media features, preferred color scheme",
    ],
    legend: &[('g', "fg #00a000")],
    grid: r#"
|range minw maxw and list or notp print only unk nunk px   |
|ggggg.gggg......ggg.gggg.gg.gggg.......gggg..........gg...|
|npx nar ex dark ld root hov ptr grd col lay nest smed sprn|
|...........gggg.gg.gggg.ggg.ggg.ggg.ggg.ggg.gggg.gggg.....|
|imp impp                                                  |
|ggg.......................................................|
"#,
};
