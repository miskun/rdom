//! Tile 31 — animations and transitions at time 0.
//!
//! Spec: CSS Animations 1 §3 (`@keyframes`), §4 (the `animation`
//! shorthand and longhands: a negative `animation-delay` starts the
//! animation that far in; `paused` holds it there; `alternate` runs the
//! second iteration backwards; `both` fills), Web Animations 1 §4.8 (the
//! iteration and directed progress), CSS Animations 2 (`animation-
//! composition: add` adds the animated value to the underlying one;
//! `replace` replaces it); Scroll-driven Animations 1 §2–§4 (a scroll
//! timeline's progress is its scroller's scroll position over its range —
//! 0 at the start; `animation-range` `normal`; the view timeline and
//! `timeline-scope` properties, whose timeline nothing here reads), CSS
//! Transitions 1 §3 (a transition starts on the style change, its negative
//! delay starting it part-way), CSS Transitions 2 §3 (`@starting-style`:
//! a newly rendered element transitions from its starting style;
//! `transition-behavior`), CSS Properties and Values 1 §3 (`@property`'s
//! `initial-value` when nothing sets the property), CSS Cascade 6 §2.5
//! (`@scope (A) to (B)`: B and its subtree outside the scope). DIVERGENCES
//! §1 "A transition moves geometry a whole cell at a time" (layout reads
//! the running value; half-way between 2 and 10 is 6).
//!
//! Derivation (the clock at 0; widths in cells):
//!
//! - Row 0: 0 → 10 over 10s, 5s in, paused: 5 (teal).
//! - Row 1: 12s into two alternating 10s iterations: the second, 2s in,
//!   backwards — 0.8 of the way: 8 (navy).
//! - Row 2: 0 → 4 half-way, added to its own 2: 4 (olive).
//! - Row 3: the bar's scroll timeline at scroll 0: its `from`, 3 (teal);
//!   the scroller (2 rows, no bar) shows only it and the empty view
//!   subject below it.
//! - Row 5: the load script sets `width: 10` on a 2-wide box with a 10s
//!   transition delayed −5s: half-way, 6 (maroon).
//! - Row 6: rendered from its starting width 2 toward 6, 5s into 10s: 4
//!   (teal).
//! - Row 7: `var()` of a registered `<length>` nothing sets: its initial
//!   3 (navy).
//! - Rows 8–9: `in` green inside the scope; `out`, inside its lower
//!   bound, default.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "31",
    spec: &[
        "CSS Animations 1 §3–§4; CSS Animations 2; Web Animations 1 §4.8; Scroll-driven Animations 1 §2–§4",
        "CSS Transitions 1 §3; CSS Transitions 2 §3; Properties and Values 1 §3; CSS Cascade 6 §2.5",
        "DIVERGENCES §1 transition geometry",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|                                                          |
|ttttt.....................................................|
|                                                          |
|nnnnnnnn..................................................|
|                                                          |
|oooo......................................................|
|                                                          |
|ttt.......................................................|
|                                                          |
|..........................................................|
|                                                          |
|mmmmmm....................................................|
|                                                          |
|tttt......................................................|
|                                                          |
|nnn.......................................................|
|in                                                        |
|gg........................................................|
|out                                                       |
|..........................................................|
"#,
};
