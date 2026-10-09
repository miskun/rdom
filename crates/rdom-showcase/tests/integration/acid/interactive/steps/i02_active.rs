//! I2 — press and release (tile 34, row 1).
//!
//! Spec: Selectors 4 §9.4 and HTML §4.16.3 — `:active` matches an
//! element "being activated": for a pointer, from the press of the
//! primary button until its release. UI Events §3.4 orders a release's
//! events `mouseup` then `click`; Blink ends the activation when it
//! processes the release, before it dispatches either
//! (`EventHandler::HandleMouseReleaseEvent`'s release hit test), so
//! neither listener sees `:active` — the order rdom pins
//! (`P7G-ACTIVE-CLEARS-ON-RELEASE-1`).
//!
//! Derivation, from tile 34's rest state:
//!
//! 1. Pointer onto `press` (x 1, row 1) and the button down: `.b` is
//!    being activated — `.b:active` fills `press` red. No listener has
//!    run (the log listens to `mouseup` and `click` only).
//! 2. Release there: the fill goes; `mouseup` then `click` each append
//!    their letter and `-` (not active): `log:u-c-`.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I2",
    title: "Press and release",
    page: 10,
    spec: &["Selectors 4 §9.4; HTML §4.16.3; UI Events §3.4"],
    run,
};

const SPEC: &[&str] = &["Selectors 4 §9.4; HTML §4.16.3; UI Events §3.4"];

static PRESSED: Reference = Reference {
    tile: "34",
    spec: SPEC,
    legend: &[('R', "bg #ff0000")],
    grid: r#"
|abkidcd sib far                       |
|......................................|
|press log:                            |
|RRRRR.................................|
|hit blk 0 0                           |
|......................................|
"#,
};

static RELEASED: Reference = Reference {
    tile: "34",
    spec: SPEC,
    legend: &[],
    grid: r#"
|abkidcd sib far                       |
|......................................|
|press log:u-c-                        |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
};

fn run(s: &mut Session) {
    s.hover("34", 1, 1);
    s.press("34", 1, 1);
    s.expect("button down on press", &[&PRESSED]);
    s.release("34", 1, 1);
    s.expect("button up: mouseup, click", &[&RELEASED]);
}
