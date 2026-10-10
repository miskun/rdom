//! I11 — wheel, keys and Tab in scroll containers (tile 41; C8-SNAP,
//! C8-OVERSCROLL, C8-SCROLL-PADDING, C8G-SNAP-TALL, C8G-RESNAP).
//!
//! Spec: CSS Scroll Snap 1 §5 (a snap container rests at a snap
//! position after a scroll; after a layout that moves the snap position
//! it rests at, it re-snaps to it, §5.4), §6.2 (`scroll-snap-stop:
//! always` — "must not pass over" that position in a directional
//! scroll), §6.2.3 (in a snap area larger than the snapport every offset
//! at which it covers the snapport is valid), §4 (`scroll-padding`
//! insets the optimal viewing region); CSS Overscroll Behavior 1 §3
//! (`contain` stops scroll chaining at its box, `auto` lets a scroll the
//! box cannot take go to its scrollable ancestor); HTML §6.6.2 / CSSOM
//! View §5.1 (Tab focuses the next element and scrolls it into view,
//! `nearest`, within the optimal viewing region); CSSOM View §12.1
//! (`scroll-behavior: smooth`). The terminal mappings rdom documents:
//! DIVERGENCES §2 "Scroll snapping decides per scroll operation, with a
//! 2-cell proximity" (a wheel tick is a 1-cell directional scroll, a
//! page key one of the viewport's height; `mandatory` rests at the snap
//! position past the start in the scroll's direction nearest its
//! destination; `proximity` snaps only within 2 cells; a directional
//! scroll from inside a tall area's covering range that would leave it
//! rests at the range's end), "Keyboard scrolling does not chain",
//! "Smooth scrolling has a fixed duration and curve" (250 ms, cubic
//! ease-out, whole cells), "Under an `App`, focusing scrolls at the next
//! layout".
//!
//! Derivation, from tile 41's rest state (each box's offset):
//!
//! 1. Two wheel ticks over the mandatory list: snap positions every row
//!    (0–7, the last item's clamped to the range), so each tick moves one
//!    item: 2.
//! 2. Click it (focus; a pointer's focus scrolls nothing), PageDown: the
//!    destination 2 + 3 = 5 would pass `m4`'s `always` position: 4.
//! 3. PageDown: destination 7, a snap position: 7.
//! 4. A tick over the proximity list (items 6 rows: positions 0, 6): the
//!    position past the start, 6, is 5 cells from the destination 1 —
//!    more than 2 — so it rests unsnapped at 1, between items.
//! 5. Two ticks over the `contain` scroller (3 lines in 2 rows): 1, then
//!    at its end the tick does not chain — the outer box stays at 0.
//! 6. Click the plain inner scroller (focus), Down twice: 1, then its
//!    end — the key does not chain; the outer box stays at 0.
//! 7. A tick over it at its end chains (`auto`): the outer box moves to 1
//!    (`c2`, `d1`, `d2`, `o3`).
//! 8. Click `t1`, Tab: `t2` (y 2) is focused; the region inside the
//!    padding is the scrollport's middle row, so `nearest` brings `t2`'s
//!    bottom edge to the region's: 1.
//! 9. Tab: `t3`: 2.
//! 10. Two ticks over the re-snapping list: 2 (`r2` at the top); the
//!     script inserts `new` before `r0`: `r2`'s snap position moves to
//!     3, and the list re-snaps to it — `r2` still at the top.
//! 11. A tick over the list with the tall card: from 0 the position past
//!     the start nearest the destination 1 is the card's start, 3 (`T0`);
//!     a second tick: 4 lies in the card's covering range (3–8), valid:
//!     4.
//! 12. Click it (focus), PageDown: 7 (valid); PageDown: the destination
//!     10 leaves the range — it rests at its end, 8; PageDown: the next
//!     item's position, 11 (`B0`).
//! 13. Click the smooth list (focus), PageDown: the destination 4, an
//!     item's position; smooth — its first frame 0, at 50 ms
//!     `round(4 · (1 − 0.8³))` = 2, at 250 ms 4.

use crossterm::event::KeyCode;
use rdom_showcase::demos::acid::tiles::t41_snap;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I11",
    title: "Wheel, keys and Tab in scroll containers",
    page: 11,
    spec: &["CSS Scroll Snap 1 §4–§6; CSS Overscroll Behavior 1 §3; CSSOM View §5.1, §12.1"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "CSS Scroll Snap 1 §4, §5, §5.4, §6.2, §6.2.3; CSS Overscroll Behavior 1 §3",
    "HTML §6.6.2; CSSOM View §5.1, §12.1",
    "DIVERGENCES §2 snapping per scroll operation, keyboard scrolling does not chain, smooth scrolling",
];

static TWO_TICKS: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m2   a0   c0   t0   r0   A0   x0      |
|......................................|
|m3   a1   c1   t1   r1   A1   x1      |
|......................................|
|m4   a2   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};

static STOP_ALWAYS: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m4   a0   c0   t0   r0   A0   x0      |
|......................................|
|m5   a1   c1   t1   r1   A1   x1      |
|......................................|
|m6   a2   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};

static PAGE_AGAIN: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a0   c0   t0   r0   A0   x0      |
|......................................|
|m8   a1   c1   t1   r1   A1   x1      |
|......................................|
|m9   a2   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};

static PROXIMITY: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c0   t0   r0   A0   x0      |
|......................................|
|m8   a2   c1   t1   r1   A1   x1      |
|......................................|
|m9   a3   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};

static CONTAIN: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c1   t0   r0   A0   x0      |
|......................................|
|m8   a2   c2   t1   r1   A1   x1      |
|......................................|
|m9   a3   d0   t2   r2   A2   x2      |
|......................................|
|          d1                  x3      |
|......................................|
"#,
};

static KEYS_NO_CHAIN: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c1   t0   r0   A0   x0      |
|......................................|
|m8   a2   c2   t1   r1   A1   x1      |
|......................................|
|m9   a3   d1   t2   r2   A2   x2      |
|......................................|
|          d2                  x3      |
|......................................|
"#,
};

static WHEEL_CHAINS: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t0   r0   A0   x0      |
|......................................|
|m8   a2   d1   t1   r1   A1   x1      |
|......................................|
|m9   a3   d2   t2   r2   A2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TAB_1: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t1   r0   A0   x0      |
|......................................|
|m8   a2   d1   t2   r1   A1   x1      |
|......................................|
|m9   a3   d2   t3   r2   A2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TAB_2: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r0   A0   x0      |
|......................................|
|m8   a2   d1   t3   r1   A1   x1      |
|......................................|
|m9   a3   d2   t4   r2   A2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static RS_TICKS: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   A0   x0      |
|......................................|
|m8   a2   d1   t3   r3   A1   x1      |
|......................................|
|m9   a3   d2   t4   r4   A2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static RS_INSERTED: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   A0   x0      |
|......................................|
|m8   a2   d1   t3   r3   A1   x1      |
|......................................|
|m9   a3   d2   t4   r4   A2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TALL_IN: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   T0   x0      |
|......................................|
|m8   a2   d1   t3   r3   T1   x1      |
|......................................|
|m9   a3   d2   t4   r4   T2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TALL_TICK: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   T1   x0      |
|......................................|
|m8   a2   d1   t3   r3   T2   x1      |
|......................................|
|m9   a3   d2   t4   r4   T3   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TALL_PAGE: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   T4   x0      |
|......................................|
|m8   a2   d1   t3   r3   T5   x1      |
|......................................|
|m9   a3   d2   t4   r4   T6   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TALL_END: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   T5   x0      |
|......................................|
|m8   a2   d1   t3   r3   T6   x1      |
|......................................|
|m9   a3   d2   t4   r4   T7   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static TALL_PAST: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   B0   x0      |
|......................................|
|m8   a2   d1   t3   r3   B1   x1      |
|......................................|
|m9   a3   d2   t4   r4   B2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static SMOOTH_0: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   B0   x0      |
|......................................|
|m8   a2   d1   t3   r3   B1   x1      |
|......................................|
|m9   a3   d2   t4   r4   B2   x2      |
|......................................|
|          o3                  x3      |
|......................................|
"#,
};

static SMOOTH_50: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   B0   x2      |
|......................................|
|m8   a2   d1   t3   r3   B1   x3      |
|......................................|
|m9   a3   d2   t4   r4   B2   y0      |
|......................................|
|          o3                  y1      |
|......................................|
"#,
};

static SMOOTH_250: Reference = Reference {
    tile: "41",
    spec: SPEC,
    legend: &[],
    grid: r#"
|m7   a1   c2   t2   r2   B0   y0      |
|......................................|
|m8   a2   d1   t3   r3   B1   y1      |
|......................................|
|m9   a3   d2   t4   r4   B2   y2      |
|......................................|
|          o3                  y3      |
|......................................|
"#,
};

fn run(s: &mut Session) {
    let wheel = |s: &mut Session, dx: u16, dy: u16, n: usize| {
        for _ in 0..n {
            s.wheel("41", dx, dy, true);
        }
    };
    wheel(s, 1, 1, 2);
    s.expect("two ticks on the mandatory list", &[&TWO_TICKS]);
    s.click("41", 1, 1);
    s.key(KeyCode::PageDown);
    s.expect("PageDown stops at the always item", &[&STOP_ALWAYS]);
    s.key(KeyCode::PageDown);
    s.expect("PageDown again", &[&PAGE_AGAIN]);
    wheel(s, 6, 1, 1);
    s.expect("a tick on the proximity list", &[&PROXIMITY]);
    wheel(s, 11, 0, 2);
    s.expect("two ticks on the contain scroller", &[&CONTAIN]);
    s.click("41", 11, 2);
    s.key(KeyCode::Down);
    s.key(KeyCode::Down);
    s.expect("Down twice in the plain scroller", &[&KEYS_NO_CHAIN]);
    wheel(s, 11, 2, 1);
    s.expect("a tick on it at its end chains", &[&WHEEL_CHAINS]);
    s.click("41", 16, 1);
    s.key(KeyCode::Tab);
    s.expect("Tab past the scroll-padding", &[&TAB_1]);
    s.key(KeyCode::Tab);
    s.expect("Tab again", &[&TAB_2]);
    wheel(s, 21, 1, 2);
    s.expect("two ticks on the re-snapping list", &[&RS_TICKS]);
    let tile = s.find_tile("41");
    s.script(|dom| t41_snap::insert_row(dom, tile));
    s.expect("a row inserted above: re-snapped", &[&RS_INSERTED]);
    wheel(s, 26, 1, 1);
    s.expect("a tick into the tall card", &[&TALL_IN]);
    wheel(s, 26, 1, 1);
    s.expect("a tick through it", &[&TALL_TICK]);
    s.click("41", 26, 1);
    s.key(KeyCode::PageDown);
    s.expect("PageDown inside it", &[&TALL_PAGE]);
    s.key(KeyCode::PageDown);
    s.expect("PageDown to its end", &[&TALL_END]);
    s.key(KeyCode::PageDown);
    s.expect("PageDown past it", &[&TALL_PAST]);
    s.click("41", 31, 1);
    s.key(KeyCode::PageDown);
    s.expect("smooth PageDown: its first frame", &[&SMOOTH_0]);
    s.advance(50);
    s.expect("smooth PageDown: 50 ms", &[&SMOOTH_50]);
    s.advance(200);
    s.expect("smooth PageDown: 250 ms", &[&SMOOTH_250]);
}
