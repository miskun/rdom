//! I10 — a `pointer-events: none` overlay (tile 34, row 2).
//!
//! Spec: CSS UI 4 §4.4 — an element with `pointer-events: none` is never
//! the target of a pointer event: the hit test passes through it to what
//! lies beneath. CSS 2.1 Appendix E / CSSOM View §6
//! (`elementFromPoint`): the hit is the topmost box at the point, so an
//! overlay without it takes the click itself. UI Events §3.5: `click`
//! goes to the hit element and bubbles through its ancestors — an
//! overlay's click never reaches the sibling box under it.
//!
//! Derivation, from tile 34's rest state:
//!
//! 1. Click at `h` (x 0, row 2): `.o1` lies over it but is
//!    `pointer-events: none`, so the hit is `.u1`; its listener counts:
//!    the first counter `1`.
//! 2. Click at `b` (x 4): `.o2` takes the hit; `.u2` sees no click, its
//!    counter stays `0`.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I10",
    title: "pointer-events: none overlay",
    page: 10,
    spec: &["CSS UI 4 §4.4; CSSOM View §6; UI Events §3.5"],
    run,
    configure: None,
};

static AFTER: Reference = Reference {
    tile: "34",
    spec: &["CSS UI 4 §4.4; UI Events §3.5"],
    legend: &[],
    grid: r#"
|abkidcd sib far                       |
|......................................|
|press log:                            |
|......................................|
|hit blk 1 0                           |
|......................................|
"#,
};

fn run(s: &mut Session) {
    s.hover("34", 0, 2);
    let u1 = s.find("34", ".u1");
    let hovered = s.dom().hovered();
    s.check_eq(
        "the hit under a pointer-events: none overlay",
        hovered,
        Some(u1),
        "CSS UI 4 §4.4",
    );
    s.click("34", 0, 2);
    s.hover("34", 4, 2);
    let o2 = s.find("34", ".o2");
    let hovered = s.dom().hovered();
    s.check_eq(
        "the hit under a plain overlay",
        hovered,
        Some(o2),
        "CSSOM View §6",
    );
    s.click("34", 4, 2);
    s.expect("after both clicks", &[&AFTER]);
}
