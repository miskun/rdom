//! I16 — popover entry and exit (tile 44; C12-STARTING, C12-BEHAVIOR).
//!
//! Spec: CSS Transitions 1 §3 — an element that was `display: none` has
//! no before-change style, so showing it starts no transition — unless
//! `@starting-style` gives one (CSS Transitions 2 §3): the popover fades
//! in from `opacity: 0`. Hiding it (HTML §6.12) removes `:popover-open`,
//! so its `opacity` goes back to 0 and `display` to `none`; under
//! `transition-behavior: allow-discrete` `display` keeps its non-`none`
//! value for the whole transition (CSS Transitions 2 §3.1), and the
//! `overlay` property (CSS Position 4 §3.4), transitioned the same way,
//! keeps it in the top layer until the end; without an `overlay`
//! transition it leaves the top layer at once, painting where its own
//! `position: fixed` puts it in its stacking context — below the tile's
//! `z-index: 5` box. CSS Color 4 §3.2 / DIVERGENCES §2 "`opacity`
//! composites per cell" (`α·src + (1 − α)·dst`; at α ≥ 0.5 the layer's
//! glyph wins; at 0 nothing is drawn).
//!
//! Derivation (100 ms linear; navy `#000080` with `#c8c8c8` text over the
//! black tile and, at x 4, the maroon box):
//!
//! 1. Click `[ f ]` (clock 0): shown, `:popover-open`, at its starting
//!    opacity 0 — nothing drawn.
//! 2. 50 ms: 0.5 — `fad` `#646464` on `#000040` (over black), `e` over
//!    the maroon box `#a46464` on `#400040`.
//! 3. 100 ms: 1 — `fade` `#c8c8c8` on navy, above the maroon box (top
//!    layer).
//! 4. Click `[ f ]` again: hidden — no longer `:popover-open` — its exit
//!    starting at opacity 1: as 3.
//! 5. 50 ms: 0.5, still in the top layer: as 2.
//! 6. 100 ms: ended — `display: none`, out of the top layer: the rest
//!    state.
//! 7. Click `[ g ]`: shown at 0 (nothing); 100 ms later fully, as 3.
//! 8. Click `[ g ]` again: hidden; out of the top layer at once — the
//!    maroon box (`z-index: 5`) over its fourth cell: `fad` on navy, x 4
//!    blank maroon.
//! 9. 50 ms: `fad` at 0.5, x 4 the maroon box.
//! 10. 100 ms: gone.

use super::super::super::reference::Reference;
use super::super::super::refs::t44_popover_motion;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I16",
    title: "Popover entry and exit",
    page: 12,
    spec: &["CSS Transitions 1 §3, 2 §3; CSS Position 4 §3.4; HTML §6.12"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "CSS Transitions 1 §3, 2 §3, §3.1; CSS Position 4 §3.4; HTML §6.12",
    "CSS Color 4 §3.2; DIVERGENCES §2 opacity per cell",
];

const LEGEND: &[(char, &str)] = &[
    ('K', "bg #000000"),
    ('b', "fg #1e90ff bg #000000 bold"),
    ('m', "bg #800000"),
    ('n', "fg #c8c8c8 bg #000080"),
    ('h', "fg #646464 bg #000040"),
    ('e', "fg #a46464 bg #400040"),
];

static HALF_TOP: Reference = Reference {
    tile: "44",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ f ] [ g ]                           |
|bbbbbKbbbbbKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
| fade                                 |
|KhhhemKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
"#,
};

static FULL: Reference = Reference {
    tile: "44",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ f ] [ g ]                           |
|bbbbbKbbbbbKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
| fade                                 |
|KnnnnmKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
"#,
};

static FULL_LOW: Reference = Reference {
    tile: "44",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ f ] [ g ]                           |
|bbbbbKbbbbbKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
| fad                                  |
|KnnnmmKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
"#,
};

static HALF_LOW: Reference = Reference {
    tile: "44",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ f ] [ g ]                           |
|bbbbbKbbbbbKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
| fad                                  |
|KhhhmmKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
|                                      |
|KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK|
"#,
};

fn run(s: &mut Session) {
    let rest = &t44_popover_motion::REF;
    let shown = |s: &Session, sel: &str| {
        let id = s.find("44", sel);
        s.dom().node(id).matches(":popover-open")
    };
    s.click("44", 2, 0);
    s.expect("f shown: opacity 0", &[rest]);
    let open = shown(s, ".pf");
    s.check_eq("f is :popover-open", open, true, "HTML §6.12");
    s.advance(50);
    s.expect("f fading in: 50 ms", &[&HALF_TOP]);
    s.advance(50);
    s.expect("f shown: 100 ms", &[&FULL]);
    s.click("44", 2, 0);
    s.expect("f hidden: its exit at opacity 1", &[&FULL]);
    let open = shown(s, ".pf");
    s.check_eq("f no longer :popover-open", open, false, "HTML §6.12");
    s.advance(50);
    s.expect("f fading out in the top layer: 50 ms", &[&HALF_TOP]);
    s.advance(50);
    s.expect("f gone: 100 ms", &[rest]);
    s.click("44", 7, 0);
    s.expect("g shown: opacity 0", &[rest]);
    s.advance(100);
    s.expect("g shown: 100 ms", &[&FULL]);
    s.click("44", 7, 0);
    s.expect("g hidden: out of the top layer at once", &[&FULL_LOW]);
    s.advance(50);
    s.expect("g fading out below the box: 50 ms", &[&HALF_LOW]);
    s.advance(50);
    s.expect("g gone: 100 ms", &[rest]);
}
