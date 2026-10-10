//! I24 — overflowing text's hit, the documented limitation (tile 53;
//! ACID-STATIC-REST, TECH_DEBT `HIT-OVERFLOW-1`).
//!
//! Spec: CSSOM View §6 — a point hits the topmost box *drawn* there, so
//! a browser hits the paragraph's overflowing text where it is painted,
//! over the float. rdom does not: DIVERGENCES §4 "Content that overflows
//! a box which does not clip it is hit only inside that box" (the hit test
//! prunes at each box's border box) — the documented behaviour this step
//! pins, so a fix of `HIT-OVERFLOW-1` turns this checkpoint (and the
//! entry) over.
//!
//! Derivation (tile 53's static reference): `overflowing` from x 0 in a
//! 6-wide paragraph, `ing` at x 8–10 over the float. At (2, 0), inside the
//! paragraph, the paragraph is hit; at (9, 0), its text over the float,
//! the float is — the browser's answer would be the paragraph.

use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I24",
    title: "Overflowing text's hit (HIT-OVERFLOW-1)",
    page: 13,
    spec: &["CSSOM View §6; DIVERGENCES §4 overflow hit, TECH_DEBT HIT-OVERFLOW-1"],
    run,
    configure: None,
};

fn run(s: &mut Session) {
    let (ov, fr) = (s.find("53", ".ov"), s.find("53", ".fr"));
    s.hover("53", 2, 0);
    let hovered = s.dom().hovered();
    s.check_eq(
        "inside its box, the paragraph",
        hovered,
        Some(ov),
        "CSSOM View §6",
    );
    s.hover("53", 9, 0);
    let hovered = s.dom().hovered();
    s.check_eq(
        "its text over the float: the float (documented)",
        hovered,
        Some(fr),
        "DIVERGENCES §4 overflow hit; HIT-OVERFLOW-1",
    );
}
