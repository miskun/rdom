//! I6 — advance the clock mid-transition (tile 37; C12-ANIMATABLE,
//! C12-TIMING).
//!
//! Spec: CSS Transitions 1 §3 — a change of a computed value starts a
//! transition at the time of the style change, its delay and duration
//! from `transition-*`; a negative delay starts it part-way (§2.4); the
//! value at a time is the timing function's output for the transition's
//! progress (Web Animations 1 §4.6–§4.8), and transitions fill backwards
//! through their delay (§3: their effects' `fill` is `backwards`).
//! CSS Easing 1 §2.2 (`linear()`: the input mapped piecewise linearly
//! between the stops `(0, 0)`, `(0.5, 0.75)`, `(1, 1)`), §2.3 (`steps(2,
//! jump-start)`: the current step `floor(p · 2) + 1`, less one with the
//! before flag at a boundary — so 0 through the delay, ½ from its end,
//! 1 from progress ½). CSS Color 4 §12.1 — "user agents must handle
//! interpolation between legacy sRGB color formats … in gamma-encoded
//! sRGB space": `rgb(0, 0, 0)` → `rgb(200, 0, 0)` is `rgb(200 · v, 0,
//! 0)`, every value here a whole byte. CSS Transitions 1 §6
//! (`transitionrun` when a transition is created, `transitionstart` when
//! its delay ends), HTML §8.1.7.3 "update the rendering" (animations are
//! updated and their events dispatched before style and layout, so a
//! listener's change shows in the same frame). DIVERGENCES §1 "A
//! transition moves geometry a whole cell at a time" (layout reads the
//! running value, rounded to whole cells, ties to even — every value here
//! exact); CSS Values 5 §11 / DIVERGENCES §2 details slot (`height: 0` →
//! `auto` under `interpolate-size: allow-keywords` is `calc-size(auto,
//! size · p)`; `content-visibility` under `allow-discrete` is `visible`
//! for the whole transition to it, CSS Containment 2 §4).
//!
//! Derivation: the script adds `.on` and opens the `details` at the
//! clock's 0 (one frame), then the clock moves to 25, 50 and 100 ms:
//!
//! | t | `linear()` | steps + 50ms delay | −50ms delay | linear | gap, padding | height | slot |
//! |---|---|---|---|---|---|---|---|
//! | 25 | 0.375 → 75 | delay → 0 | 0.75 → 150 | 0.25 → 50 | 1 | 2 | 1 row |
//! | 50 | 0.75 → 150 | p 0 → ½ → 100 | ended → 200 | 0.5 → 100 | 2 | 3 | 2 rows |
//! | 100 | 1 → 200 | p ½ → 1 → 200 | 200 | 200 | 4 | 5 | 4 rows |
//!
//! The log reads `log:r` at 25 (the run event of the frame that created
//! the transition), `log:rs` from 50 (the delay ended that frame). The
//! row-3 box and the `details` (`▾ s`, its slot rows `1` … under it) grow
//! together, and `end` sits under the taller: rows 5, 6, 8.

use rdom_showcase::demos::acid::tiles::t37_transitions;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I6",
    title: "Advance the clock mid-transition",
    page: 10,
    spec: &["CSS Transitions 1 §3, §6; CSS Easing 1 §2.2–§2.3; CSS Color 4 §12.1"],
    run,
};

const SPEC: &[&str] = &[
    "CSS Transitions 1 §2.4, §3, §6; Web Animations 1 §4.6–§4.8",
    "CSS Easing 1 §2.2, §2.3; CSS Color 4 §12.1 (legacy colors in sRGB)",
    "CSS Values 5 §11; CSS Containment 2 §4; DIVERGENCES §1 whole-cell geometry",
];

static T25: Reference = Reference {
    tile: "37",
    spec: SPEC,
    legend: &[
        ('a', "bg #4b0000"),
        ('K', "bg #000000"),
        ('b', "bg #960000"),
        ('c', "bg #320000"),
        ('n', "bg #000080"),
    ],
    grid: r#"
|                    log:r                                 |
|aaaa.KKKK.bbbb.cccc.......................................|
|a b c                                                     |
|..........................................................|
| pad                                                      |
|..........................................................|
|     ▾ s                                                  |
|nnnn......................................................|
|     1                                                    |
|nnnn......................................................|
|end                                                       |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};

static T50: Reference = Reference {
    tile: "37",
    spec: SPEC,
    legend: &[
        ('b', "bg #960000"),
        ('d', "bg #640000"),
        ('e', "bg #c80000"),
        ('n', "bg #000080"),
    ],
    grid: r#"
|                    log:rs                                |
|bbbb.dddd.eeee.dddd.......................................|
|a  b  c                                                   |
|..........................................................|
|  pad                                                     |
|..........................................................|
|     ▾ s                                                  |
|nnnn......................................................|
|     1                                                    |
|nnnn......................................................|
|     2                                                    |
|nnnn......................................................|
|end                                                       |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};

static T100: Reference = Reference {
    tile: "37",
    spec: SPEC,
    legend: &[('e', "bg #c80000"), ('n', "bg #000080")],
    grid: r#"
|                    log:rs                                |
|eeee.eeee.eeee.eeee.......................................|
|a    b    c                                               |
|..........................................................|
|    pad                                                   |
|..........................................................|
|     ▾ s                                                  |
|nnnn......................................................|
|     1                                                    |
|nnnn......................................................|
|     2                                                    |
|nnnn......................................................|
|     3                                                    |
|nnnn......................................................|
|     4                                                    |
|nnnn......................................................|
|end                                                       |
|..........................................................|
"#,
};

fn run(s: &mut Session) {
    let tile = s.find_tile("37");
    s.script(|dom| t37_transitions::start(dom, tile));
    s.advance(25);
    s.expect("25 ms", &[&T25]);
    s.advance(25);
    s.expect("50 ms", &[&T50]);
    s.advance(50);
    s.expect("100 ms", &[&T100]);
}
