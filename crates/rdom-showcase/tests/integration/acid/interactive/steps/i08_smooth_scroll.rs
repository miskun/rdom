//! I8 — smooth scroll and `scrollIntoView` (tile 39).
//!
//! Spec: CSSOM View §12.1 — `scroll-behavior: smooth` makes a
//! programmatic scroll of the box (behavior `auto`) smooth; §4.1 "perform
//! a scroll": a smooth scroll moves "in a user-agent-defined fashion
//! within a user-agent-defined amount of time" — rdom's, DIVERGENCES §2
//! "Smooth scrolling has a fixed duration and curve": 250 ms, cubic
//! ease-out `1 − (1 − t)³`, positions rounded to whole cells, starting at
//! the frame after the request (where `t` is 0). §5.1 "scroll an element
//! into view": every scrolling ancestor, innermost first, scrolls the
//! element's box to the `block` position — `nearest` leaves a box whose
//! element is already wholly visible where it is, so the outer box,
//! whose scrollport shows the inner box entire, is untouched; each box
//! scrolls with its own behavior (`auto` → its `scroll-behavior`).
//!
//! Derivation, from tile 39's rest state (inner box at 0, outer at 0):
//!
//! 1. `inner.scrollTo({top: 8})`, then its first frame: t = 0, still 0.
//! 2. +125 ms: t = ½, eased 0.875 → 7: `a7`–`a9`.
//! 3. +125 ms: settled at 8: `a8`–`a10`. The outer box still shows `top`
//!    and `o2` (0).
//! 4. `a2.scrollIntoView({block: "nearest"})`: `a2` is above the inner
//!    scrollport, so the nearer edge aligns — its top, offset 2; the
//!    inner box is wholly in the outer's scrollport, which stays at 0.
//!    First frame: t = 0, 8.
//! 5. +125 ms: 8 + (2 − 8) · 0.875 = 2.75 → 3: `a3`–`a5`.
//! 6. +125 ms: settled at 2: `a2`–`a4`; the outer box at 0 throughout.

use rdom_tui::{
    ScrollIntoViewOptions, ScrollLogicalPosition, ScrollToOptions, TuiAccessors, TuiAccessorsMut,
};

use super::super::super::reference::Reference;
use super::super::super::refs::t39_smooth_scroll;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I8",
    title: "Smooth scroll and scrollIntoView",
    page: 10,
    spec: &["CSSOM View §4.1, §5.1, §12.1; DIVERGENCES §2 smooth scrolling"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "CSSOM View §4.1, §5.1, §12.1",
    "DIVERGENCES §2 Smooth scrolling has a fixed duration and curve",
];

macro_rules! inner_at {
    ($name:ident, $a:literal, $b:literal, $c:literal) => {
        static $name: Reference = Reference {
            tile: "39",
            spec: SPEC,
            legend: &[('n', "bg #000080")],
            grid: concat!(
                "|top                                   |\n",
                "|......................................|\n",
                "|",
                $a,
                "|\n",
                "|nnnnnn................................|\n",
                "|",
                $b,
                "|\n",
                "|nnnnnn................................|\n",
                "|",
                $c,
                "|\n",
                "|nnnnnn................................|\n",
                "|o2                                    |\n",
                "|......................................|\n",
            ),
        };
    };
}

inner_at!(
    AT_7,
    "a7                                    ",
    "a8                                    ",
    "a9                                    "
);
inner_at!(
    AT_8,
    "a8                                    ",
    "a9                                    ",
    "a10                                   "
);
inner_at!(
    AT_3,
    "a3                                    ",
    "a4                                    ",
    "a5                                    "
);
inner_at!(
    AT_2,
    "a2                                    ",
    "a3                                    ",
    "a4                                    "
);

fn run(s: &mut Session) {
    let (out, inner, tg) = (
        s.find("39", ".out"),
        s.find("39", ".in"),
        s.find("39", ".tg"),
    );
    s.script(|dom| {
        dom.node_mut(inner)
            .scroll_with(ScrollToOptions::new().top(8))
            .unwrap();
    });
    s.expect(
        "scrollTo requested: its first frame",
        &[&t39_smooth_scroll::REF],
    );
    s.advance(125);
    s.expect("125 ms", &[&AT_7]);
    s.advance(125);
    s.expect("250 ms: settled", &[&AT_8]);
    s.script(|dom| {
        let options = ScrollIntoViewOptions::new().block(ScrollLogicalPosition::Nearest);
        dom.node_mut(tg).scroll_into_view_with(options).unwrap();
    });
    s.expect("scrollIntoView requested: its first frame", &[&AT_8]);
    s.advance(125);
    s.expect("125 ms", &[&AT_3]);
    s.advance(125);
    s.expect("250 ms: settled", &[&AT_2]);
    let outer_top = s.dom().node(out).scroll_top();
    s.check_eq(
        "the outer box untouched",
        outer_top,
        Some(0),
        "CSSOM View §5.1",
    );
}
