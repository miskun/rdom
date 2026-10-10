//! I20 — slide a panel in (tile 48; C15-TRANSLATE).
//!
//! Spec: CSS Transforms 2 §5.1 (`translate`, percentages of the border
//! box), CSS Transforms 1 §3 (`transform`; a transform moves the
//! rendering and no layout), CSS Transitions 1 §3 (`translate: -100% 0`
//! → `0` over 200 ms, linear), CSS Animations 1 §3 (a `@keyframes`
//! `translateX(-100%)` → `translateX(0)` over the same 200 ms, then the
//! element's own `transform: none`); CSSOM View §6 / CSS Transforms 1 §3
//! (hit-testing follows the moved box: where it is drawn, not where it
//! will be). DIVERGENCES §2 "A transform moves a box by whole cells, and
//! does nothing else" (the translation rounded once, ties to even; a
//! `rotate()` an identity: the spinner never moves). Which frames lay out
//! (only those whose cell offset changed) is not visible in cells;
//! rdom-tui's own tests pin it.
//!
//! Derivation (the panel and the box 10 wide; the click at the clock's 0):
//!
//! 1. Click `[ go ]`: `.open` — both at −10, wholly off the tile: the
//!    rest state.
//! 2. 100 ms: −5 — `56789` (navy) and `fghij` (teal) at x 0–4; the
//!    pointer at x 2 of row 1 is over the panel, at x 7 (where it is
//!    going) it is not.
//! 3. 200 ms: 0 — the whole panel and box; the spinner `S` at x 12
//!    throughout.

use super::super::super::reference::Reference;
use super::super::super::refs::t48_slide;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I20",
    title: "Slide a panel in",
    page: 6,
    spec: &["CSS Transforms 1 §3, 2 §5.1; CSS Transitions 1 §3; CSS Animations 1 §3"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "CSS Transforms 1 §3; CSS Transforms 2 §5.1; CSS Transitions 1 §3; CSS Animations 1 §3",
    "CSSOM View §6; DIVERGENCES §2 transform",
];

static HALF: Reference = Reference {
    tile: "48",
    spec: SPEC,
    legend: &[
        ('B', "fg #1e90ff bold"),
        ('n', "bg #000080"),
        ('t', "bg #008080"),
    ],
    grid: r#"
|[ go ]                                                    |
|BBBBBB....................................................|
|56789                                                     |
|nnnnn.....................................................|
|fghij       S                                             |
|ttttt.....................................................|
"#,
};

static FULL: Reference = Reference {
    tile: "48",
    spec: SPEC,
    legend: &[
        ('B', "fg #1e90ff bold"),
        ('n', "bg #000080"),
        ('t', "bg #008080"),
    ],
    grid: r#"
|[ go ]                                                    |
|BBBBBB....................................................|
|0123456789                                                |
|nnnnnnnnnn................................................|
|abcdefghij  S                                             |
|tttttttttt................................................|
"#,
};

fn run(s: &mut Session) {
    s.click("48", 2, 0);
    s.expect("clicked: both at −10", &[&t48_slide::REF]);
    s.advance(100);
    s.expect("100 ms: half-way", &[&HALF]);
    let sl = s.find("48", ".sl");
    s.hover("48", 2, 1);
    let hovered = s.dom().hovered();
    s.check_eq(
        "its visible part is hit",
        hovered,
        Some(sl),
        "CSSOM View §6",
    );
    s.hover("48", 7, 1);
    let hovered = s.dom().hovered();
    s.check(
        "where it will be is not",
        hovered != Some(sl),
        format!("{hovered:?}"),
        "CSSOM View §6; CSS Transforms 1 §3",
    );
    s.advance(100);
    s.expect("200 ms: in", &[&FULL]);
}
