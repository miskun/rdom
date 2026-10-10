//! I17 — advance the clock mid-animation (tile 45; C12-KEYFRAMES).
//!
//! Spec: CSS Animations 1 §3 (`@keyframes`: the value between two
//! keyframes interpolated by the easing of the keyframe that starts the
//! interval — its own `animation-timing-function`, else the animation's;
//! a missing `from` / `to` is the element's own value), §4
//! (`animation-*`: `alternate` runs every second iteration backwards,
//! `forwards` holds the last keyframe after the end, `backwards` the
//! first through the delay; `animation-composition: add` adds the
//! keyframe value to the underlying one, CSS Animations 2 §3.1), §4.2 /
//! CSS Animations 2 §4.2 (events: `animationstart`, `animationiteration`
//! with `elapsedTime` the iterations run, `animationend`;
//! `animationcancel` when the animation is removed while running); Web
//! Animations 1 §4.5–§4.8 (phases and progress), §5.4 (the effect stack:
//! CSS animations above CSS transitions); CSS Cascade 4 §6.1 (an
//! `!important` author declaration beats the animation origin, so it does
//! not animate); CSS Easing 1 §2.3 (`steps(2, jump-end)`); DIVERGENCES §1
//! whole-cell geometry; HTML §8.1.7.3 with ACID-FIX-15 (an event reached
//! at a frame's time is dispatched before it draws; one raised by that
//! frame's own style change — a start, a cancel — after it, so its
//! listener's change shows a frame later, as in a browser, whose next
//! frame dispatches it).
//!
//! Derivation: `.run` added at the clock's 0; the frames at 0, 25, 75,
//! 125 and 225 ms (the kca class removed at 25, after that checkpoint):
//!
//! | box | 0 | 25 | 75 | 125 | 225 |
//! |---|---|---|---|---|---|
//! | k1: 0 → 100 (steps(2)) → 200 red | 0 | 50 | 150 | own grey | grey |
//! | k2: 0 → 200 green, 2 alternate | 0 | 50 | 150 | 150 (back) | grey |
//! | k3: 40 → 200 blue, forwards | 40 | 80 | 160 | 200 | 200 |
//! | k4: k3 after 50 ms, backwards | 40 | 40 | 80 | 160 | grey |
//! | kt: red animation over a 200 ms grey → blue transition | red | red | red | `(24, 24, 149)` | blue |
//! | kc: k3, cancelled at 25 | 40 | 80 | grey | grey | grey |
//! | width 2 → 10 | 2 | 4 | 8 | own 1 | 1 |
//! | `!important` width | 3 | 3 | 3 | 3 | 3 |
//! | add 0 → 4 to 4, forwards | 4 | 5 | 7 | 8 | 8 |
//!
//! The logs: `s` from 25 (the start raised at 0), `i0.1` from 125 (the
//! iteration at 100), `e0.2` at 225 (the end at 200); `c` from 75.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I17",
    title: "Advance the clock mid-animation",
    page: 12,
    spec: &["CSS Animations 1 §3–§4, 2 §3.1, §4.2; Web Animations 1 §4–§5"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "CSS Animations 1 §3, §4, §4.2; CSS Animations 2 §3.1, §4.2",
    "Web Animations 1 §4.5–§4.8, §5.4; CSS Cascade 4 §6.1; CSS Easing 1 §2.3",
    "DIVERGENCES §1 whole-cell geometry; HTML §8.1.7.3",
];

const LEGEND: &[(char, &str)] = &[
    ('G', "bg #404040"),
    ('K', "bg #000000"),
    ('a', "bg #320000"),
    ('b', "bg #960000"),
    ('c', "bg #003200"),
    ('d', "bg #009600"),
    ('e', "bg #000028"),
    ('f', "bg #000050"),
    ('g', "bg #0000a0"),
    ('h', "bg #0000c8"),
    ('r', "bg #c80000"),
    ('t', "bg #181895"),
    ('n', "bg #000080"),
    ('T', "bg #008080"),
];

static T0: Reference = Reference {
    tile: "45",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                                          |
|KKKK.KKKK.eeee.eeee.rrrr.eeee.............................|
|                                                          |
|nn........................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTT......................................................|
|log:                                                      |
|..........................................................|
|log:                                                      |
|..........................................................|
"#,
};

static T25: Reference = Reference {
    tile: "45",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                                          |
|aaaa.cccc.ffff.eeee.rrrr.ffff.............................|
|                                                          |
|nnnn......................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTTT.....................................................|
|log:s                                                     |
|..........................................................|
|log:                                                      |
|..........................................................|
"#,
};

static T75: Reference = Reference {
    tile: "45",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                                          |
|bbbb.dddd.gggg.ffff.rrrr.GGGG.............................|
|                                                          |
|nnnnnnnn..................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTTTTT...................................................|
|log:s                                                     |
|..........................................................|
|log:c                                                     |
|..........................................................|
"#,
};

static T125: Reference = Reference {
    tile: "45",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                                          |
|GGGG.dddd.hhhh.gggg.tttt.GGGG.............................|
|                                                          |
|n.........................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTTTTTT..................................................|
|log:si0.1                                                 |
|..........................................................|
|log:c                                                     |
|..........................................................|
"#,
};

static T225: Reference = Reference {
    tile: "45",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|                                                          |
|GGGG.GGGG.hhhh.GGGG.hhhh.GGGG.............................|
|                                                          |
|n.........................................................|
|                                                          |
|nnn.......................................................|
|                                                          |
|TTTTTTTT..................................................|
|log:si0.1e0.2                                             |
|..........................................................|
|log:c                                                     |
|..........................................................|
"#,
};

fn run(s: &mut Session) {
    let tile = s.find_tile("45");
    s.script(|dom| dom.add_class(tile, "run").unwrap());
    s.expect("0 ms", &[&T0]);
    s.advance(25);
    s.expect("25 ms", &[&T25]);
    let kc = s.find("45", ".kc");
    s.script(|dom| {
        dom.remove_class(kc, "kca").unwrap();
    });
    s.advance(50);
    s.expect("75 ms", &[&T75]);
    s.advance(50);
    s.expect("125 ms", &[&T125]);
    s.advance(100);
    s.expect("225 ms", &[&T225]);
}
