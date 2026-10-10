//! I19 — resize the terminal (tiles 20, 22, 47; C14-MEDIA,
//! C14-CONTAINER, C14-CONTAIN).
//!
//! Spec: Media Queries 4 §4 (width queries against the viewport; CSSOM
//! View §4.2 `matchMedia`: a `change` event each time the list's result
//! flips, at the rendering update); CSS Conditional 5 §6 (a container
//! query re-evaluates when its container's size changes); CSS Values 4
//! §6.1.2 (`vw` follows the viewport); CSS Containment 2 §4.4
//! (`content-visibility: auto`: relevance by intersecting the viewport,
//! `contentvisibilityautostatechange` when it changes); DIVERGENCES §1
//! "The viewport is the terminal", §2 "Media features answer for a
//! terminal" (a pixel value 8px a column: `600px` is 75), "`content-
//! visibility: auto` judges relevance by the viewport" (no margin; the
//! event after the frame that painted it, so its listener's change shows
//! a frame later). How many cascades a resize costs (one whole-tree
//! cascade per flip, none for a resize that flips nothing) is not visible
//! in cells; rdom-tui's own tests pin it (`media_tests`).
//!
//! Derivation:
//!
//! 1. 120 × 50 → 70 × 50. Tile 20: `minw` (≥ 100) and `px` (≥ 75) stop
//!    matching, `maxw` (≤ 99) and `npx` start — the rest as at 120.
//!    `matchMedia("(width < 100)")` reports `true`. Tile 22: the pane is
//!    `calc(20vw + 12)` = 26 — under 30, its card a column (`IMG` /
//!    `text`), its `50cqw` bar 13. Tile 47: the `40vw` paragraph is 28
//!    wide, 9 words a line, 5 lines (rows 28–32, the tile showing 4): the
//!    `auto` block moves to page row 51, off the 50-row screen — skipped;
//!    the frame after, the log reads `log:ft`.
//! 2. Back to 120 × 50: tiles 20 and 22 as at rest; `matchMedia` reports
//!    `false`; the block back on row 49 — relevant: `log:ftf`.

use std::cell::RefCell;
use std::rc::Rc;

use super::super::super::reference::Reference;
use super::super::super::refs::{t20_media, t22_container};
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I19",
    title: "Resize the terminal",
    page: 6,
    spec: &["Media Queries 4 §4; CSSOM View §4.2; CSS Conditional 5 §6; CSS Containment 2 §4.4"],
    run,
    configure: None,
};

const SPEC_20: &[&str] = &["Media Queries 4 §4; DIVERGENCES §2 media features (70 columns)"];
const SPEC_22: &[&str] = &["CSS Conditional 5 §6; CSS Values 4 §6.1.2; DIVERGENCES §1 viewport"];
const SPEC_47: &[&str] = &["CSS Containment 2 §4.4; DIVERGENCES §2 content-visibility: auto"];

static NARROW_20: Reference = Reference {
    tile: "20",
    spec: SPEC_20,
    legend: &[('g', "fg #00a000")],
    grid: r#"
|range minw maxw and list or notp print only unk nunk px   |
|ggggg......gggg.ggg.gggg.gg.gggg.......gggg...............|
|npx nar ex dark ld root hov ptr grd col lay nest smed sprn|
|ggg........gggg.gg.gggg.ggg.ggg.ggg.ggg.ggg.gggg.gggg.....|
|imp impp                                                  |
|ggg.......................................................|
"#,
};

static NARROW_22: Reference = Reference {
    tile: "22",
    spec: SPEC_22,
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('g', "fg #00a000"),
        ('m', "bg #800000"),
        ('y', "fg #ffff00 bg #000080"),
    ],
    grid: r#"
|IMG                   IMG                                 |
|..........................................................|
|text                  text                                |
|..........................................................|
|                                                          |
|tttttttttt............ttttttttttttt.......................|
|                                                          |
|..........................................................|
|named near                tall        unk       sty       |
|ggggg.....................gggg..................ggg.......|
|                                                          |
|..........................nnn.............................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|abcd            1a 1b 2c  ┌────┐  N                       |
|mmmm..nnnnnnnn....................ynnn....................|
|       F                  └────┘                          |
|......ntnnnnnn............................................|
"#,
};

static NARROW_47: Reference = Reference {
    tile: "47",
    spec: SPEC_47,
    legend: &[],
    grid: r#"
|log:ft                                                    |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|ab ab ab ab ab ab ab ab ab                                |
|..........................................................|
|ab ab ab ab ab ab ab ab ab                                |
|..........................................................|
|ab ab ab ab ab ab ab ab ab                                |
|..........................................................|
|ab ab ab ab ab ab ab ab ab                                |
|..........................................................|
"#,
};

static WIDE_47: Reference = Reference {
    tile: "47",
    spec: SPEC_47,
    legend: &[],
    grid: r#"
|log:ftf                                                   |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab           |
|..........................................................|
|ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab           |
|..........................................................|
|ab ab ab ab ab ab ab ab                                   |
|..........................................................|
|cv                                                        |
|..........................................................|
"#,
};

fn run(s: &mut Session) {
    let log: Rc<RefCell<Vec<bool>>> = Rc::default();
    let seen = log.clone();
    s.app()
        .match_media("(width < 100)")
        .add_listener(move |_, event| seen.borrow_mut().push(event.matches));
    s.resize(70, 50);
    s.expect("70 × 50", &[&NARROW_20, &NARROW_22]);
    s.advance(0);
    s.expect("70 × 50, the frame after", &[&NARROW_47]);
    let flips = log.borrow().clone();
    s.check_eq("matchMedia at 70", flips, vec![true], "CSSOM View §4.2");
    s.resize(120, 50);
    s.expect("120 × 50 again", &[&t20_media::REF, &t22_container::REF]);
    s.advance(0);
    s.expect("120 × 50, the frame after", &[&WIDE_47]);
    let flips = log.borrow().clone();
    s.check_eq(
        "matchMedia back at 120",
        flips,
        vec![true, false],
        "CSSOM View §4.2",
    );
}
