//! I23 — a fragmented paragraph's rects and hits (tile 25;
//! ACID-STATIC-REST).
//!
//! Spec: CSSOM View §6.1 (`getClientRects()`: one rect per box fragment,
//! here the paragraph's two — one per column); CSS Fragmentation 3 §5.4
//! (a box across a column break is drawn as its fragments, nothing
//! between them); CSSOM View §6 / UI Events (a point hits the topmost box
//! drawn there: on the second fragment, the paragraph; in its bounding
//! box between the fragments — the column rule — the article);
//! DIVERGENCES §2 "`client_rects()` lists one rect per box fragment".
//!
//! Derivation (tile 25 at page (1, 1); its static reference): the red
//! paragraph's first fragment is column 1's row 3, x 1–8; its second
//! column 2's row 1, x 10–17 — page rects (2, 4, 8 × 1) and (11, 2, 8 ×
//! 1). The point (11, 1) — `2` of `r2` — hits the paragraph; (9, 2), the
//! rule between the columns inside the fragments' bounding box, hits the
//! article.

use rdom_tui::{LayoutRect, TuiAccessors};

use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I23",
    title: "A fragmented paragraph's rects and hits",
    page: 7,
    spec: &["CSSOM View §6, §6.1; CSS Fragmentation 3 §5.4"],
    run,
    configure: None,
};

fn run(s: &mut Session) {
    let (a1, sp) = (s.find("25", ".a1"), s.find("25", ".a1 .sp"));
    let rects = s.dom().node(sp).client_rects();
    s.check_eq(
        "client_rects() lists the two fragments",
        rects,
        vec![LayoutRect::new(2, 4, 8, 1), LayoutRect::new(11, 2, 8, 1)],
        "CSSOM View §6.1; DIVERGENCES §2 client_rects",
    );
    s.hover("25", 11, 1);
    let hovered = s.dom().hovered();
    s.check_eq(
        "the second fragment is hit",
        hovered,
        Some(sp),
        "CSSOM View §6",
    );
    s.hover("25", 9, 2);
    let hovered = s.dom().hovered();
    s.check_eq(
        "between the fragments, the article",
        hovered,
        Some(a1),
        "CSSOM View §6; CSS Fragmentation 3 §5.4",
    );
}
