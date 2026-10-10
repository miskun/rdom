//! I18 — scroll a scroll-driven progress bar (tile 46; C12-SCROLL-DRIVEN).
//!
//! Spec: Scroll-driven Animations 1 §2.1 — a scroll progress timeline's
//! progress is the scroll offset over the scroll range, from the scroll
//! origin (under `rtl` the inline origin is the right edge, CSSOM View
//! §4: `scrollLeft` runs 0 → −range); the end of the range is 100%, still
//! in the active interval (§2.1, CSS Animations 2 §3.3: `auto` duration);
//! no clock is needed — scrolling moves it. §2.1.2: a timeline whose
//! source is not a scroll container is inactive, its animation idle — the
//! element shows its own value. §2.2 / §4.2: `scroll-timeline` names a
//! timeline, `timeline-scope` makes it visible outside its scroller. §3.1
//! / §3.2: `view()` — the subject's crossing of the scrollport; `cover`
//! from its start edge meeting the port's end edge to its end edge
//! meeting the port's start edge, `entry` the part of it until the subject
//! is wholly inside, `contain` the part while it is; §4.3
//! `animation-range` attaches the animation to a named range, §4.4 a
//! keyframe at `contain 50%` sits at that point of the attachment range.
//! CSS Overflow 3 §3.1 (`overflow: clip` is no scroll container); CSS
//! Position 3 §3.4 (`sticky`: the bar stays at the scrollport's top);
//! CSS Color 4 §3.2 / DIVERGENCES §2 "`opacity` composites per cell";
//! DIVERGENCES §1 whole-cell geometry.
//!
//! Derivation (the scroller 4 rows over 16: range 12; the `rtl` one 12
//! cells over 24: range 12). The two-row item at rows 6–7: `entry` is
//! offsets 2–4. The one-row item at row 10: `cover` is 6–11, `contain`
//! 7–10, so `contain 50%` (8.5) is half-way through `cover` — width 2 → 8
//! → 2 (its own, the implicit end keyframes).
//!
//! 1. Offset 3, the `rtl` one scrolled 4 from its origin: the bar 3 wide
//!    (sticky over `r3`); the item half-way through `entry`, opacity 0.5:
//!    `#000040` with `v1` `#646464`; the olive bar a third: 4.
//! 2. Offset 4 (a third): the bar 4; the item through `entry`: opacity 1.
//! 3. Offset 8: the bar 8; the second item 0.4 through `cover`: 2 + 6 ·
//!    0.8 = 6.8 → 7 cells.
//! 4. Offset 12, the `rtl` one at its end: both bars 12 — the end still
//!    active.
//! 5. The scroller made `overflow: clip`: no scroll container — the bar's
//!    timeline inactive, the bar idle at its own width 1, the content from
//!    its top; the olive bar still 12.

use rdom_tui::TuiAccessorsMut;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I18",
    title: "Scroll a scroll-driven progress bar",
    page: 12,
    spec: &["Scroll-driven Animations 1 §2–§4; CSSOM View §4"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "Scroll-driven Animations 1 §2.1, §2.1.2, §2.2, §3.1, §3.2, §4.2–§4.4",
    "CSSOM View §4; CSS Overflow 3 §3.1; CSS Position 3 §3.4; CSS Color 4 §3.2",
    "DIVERGENCES §1 whole-cell geometry; §2 opacity per cell",
];

const LEGEND: &[(char, &str)] = &[
    ('t', "bg #008080"),
    ('n', "fg #c8c8c8 bg #000080"),
    ('N', "bg #000080"),
    ('h', "fg #646464 bg #000040"),
    ('H', "bg #000040"),
    ('m', "bg #800080"),
    ('o', "bg #808000"),
];

static AT_3: Reference = Reference {
    tile: "46",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                      |
|ttt...................................|
|r4                                    |
|......................................|
|r5                                    |
|......................................|
|v1                                    |
|hhHHHHHHHHHHHH........................|
|                                      |
|......................................|
|                                      |
|......................................|
|                                      |
|oooo..................................|
"#,
};

static AT_4: Reference = Reference {
    tile: "46",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                      |
|tttt..................................|
|r5                                    |
|......................................|
|v1                                    |
|nnNNNNNNNNNNNN........................|
|                                      |
|NNNNNNNNNNNNNN........................|
|                                      |
|......................................|
|                                      |
|......................................|
|                                      |
|oooo..................................|
"#,
};

static AT_8: Reference = Reference {
    tile: "46",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                      |
|tttttttt..............................|
|r9                                    |
|......................................|
|                                      |
|mmmmmmm...............................|
|r11                                   |
|......................................|
|                                      |
|......................................|
|                                      |
|......................................|
|                                      |
|oooo..................................|
"#,
};

static AT_12: Reference = Reference {
    tile: "46",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                      |
|tttttttttttt..........................|
|r13                                   |
|......................................|
|r14                                   |
|......................................|
|r15                                   |
|......................................|
|                                      |
|......................................|
|                                      |
|......................................|
|                                      |
|oooooooooooo..........................|
"#,
};

static FLAT: Reference = Reference {
    tile: "46",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                      |
|t.....................................|
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
|oooooooooooo..........................|
"#,
};

fn run(s: &mut Session) {
    let (sc, hs) = (s.find("46", ".sc"), s.find("46", ".hs"));
    let scroll = |s: &mut Session, top: i32, left: Option<i32>| {
        s.script(|dom| {
            dom.node_mut(sc).set_scroll_top(top).unwrap();
            if let Some(left) = left {
                dom.node_mut(hs).set_scroll_left(left).unwrap();
            }
        });
    };
    scroll(s, 3, Some(-4));
    s.expect("offset 3, the rtl scroller 4 from its origin", &[&AT_3]);
    scroll(s, 4, None);
    s.expect("offset 4", &[&AT_4]);
    scroll(s, 8, None);
    s.expect("offset 8", &[&AT_8]);
    scroll(s, 12, Some(-12));
    s.expect("both at their ends", &[&AT_12]);
    s.script(|dom| dom.add_class(sc, "flat").unwrap());
    s.expect("the scroller's overflow removed", &[&FLAT]);
}
