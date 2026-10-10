//! I22 — a click on a clipped-out corner (tile 24; ACID-STATIC-REST).
//!
//! Spec: CSS Masking 1 §5 — `clip-path`: "the area outside the clipping
//! region … must not receive pointer events": a point there hits what lies
//! beneath. UI Events §3.5 (`click` at the hit element). DIVERGENCES §2 "A
//! clip path clips whole cells" (a cell is inside when its centre is).
//!
//! Derivation: tile 24's teal box (x 7–16, rows 4–9) under `inset(0 round
//! 3)` has its corner cells cut (its static reference): at (7, 4) — the
//! top-left corner — the hit is the box's parent band, beneath it; a click
//! there goes to the band, not the box. At (8, 4) the box is hit.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_tui::{ListenerOptions, NodeId};

use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I22",
    title: "A click on a clipped-out corner",
    page: 6,
    spec: &["CSS Masking 1 §5; UI Events §3.5"],
    run,
    configure: None,
};

fn run(s: &mut Session) {
    let ins = s.find("24", ".ins");
    let band = s
        .dom()
        .node(ins)
        .parent_element()
        .expect("the box's band")
        .id();
    let targets: Rc<RefCell<Vec<NodeId>>> = Rc::default();
    let seen = targets.clone();
    s.app()
        .dom_mut()
        .add_event_listener(band, "click", ListenerOptions::default(), move |ctx| {
            seen.borrow_mut().push(ctx.event.target.expect("a target"));
        })
        .unwrap();
    s.hover("24", 7, 4);
    let hovered = s.dom().hovered();
    s.check_eq(
        "the cut corner hits the band beneath",
        hovered,
        Some(band),
        "CSS Masking 1 §5",
    );
    s.click("24", 7, 4);
    let clicked = targets.borrow().clone();
    s.check_eq(
        "a click there goes to the band",
        clicked,
        vec![band],
        "CSS Masking 1 §5; UI Events §3.5",
    );
    s.hover("24", 8, 4);
    let hovered = s.dom().hovered();
    s.check_eq(
        "beside the corner the box is hit",
        hovered,
        Some(ins),
        "CSS Masking 1 §5",
    );
}
