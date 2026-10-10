//! I5 — toggle a checkbox and a radio group (tile 36, row 1).
//!
//! Spec: HTML §4.10.5.1.15 / .16 — a click on a checkbox toggles its
//! checkedness (its activation behaviour); a click on an unchecked radio
//! checks it and unchecks every other radio of **its group**: the radios
//! of the same name in the same tree whose form owner is the same — the
//! radio outside the form has another (none), so it stays checked;
//! Selectors 4 §14.3 (`:checked`). A clicked toggle is focused by
//! pointer, which is not evident (DIVERGENCES `FOCUS-VOCAB-1`): no tint.
//!
//! Derivation, from tile 36's rest state:
//!
//! 1. Click the checkbox (x 1): `[x] `, green.
//! 2. Click the second radio (x 9): it is `(•) ` green, the form's first
//!    radio `( ) ` in the default colour, the outside radio still `(•) `
//!    green.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I5",
    title: "Toggle a checkbox and a radio group",
    page: 10,
    spec: &["HTML §4.10.5.1.15, §4.10.5.1.16; Selectors 4 §14.3"],
    run,
    configure: None,
};

const SPEC: &[&str] = &["HTML §4.10.5.1.15–16 (radio button group); Selectors 4 §14.3"];

static CHECKED_BOX: Reference = Reference {
    tile: "36",
    spec: SPEC,
    legend: &[
        ('f', "bg #1f2123"),
        ('r', "fg #ff0000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|      bad form                        |
|fffff.rrr.rrrr........................|
|[x] (•) ( ) (•)                       |
|gggggggg....gggg......................|
"#,
};

static SECOND_RADIO: Reference = Reference {
    tile: "36",
    spec: SPEC,
    legend: &[
        ('f', "bg #1f2123"),
        ('r', "fg #ff0000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|      bad form                        |
|fffff.rrr.rrrr........................|
|[x] ( ) (•) (•)                       |
|gggg....gggggggg......................|
"#,
};

fn run(s: &mut Session) {
    s.click("36", 1, 1);
    s.expect("click the checkbox", &[&CHECKED_BOX]);
    s.click("36", 9, 1);
    s.expect("click the second radio", &[&SECOND_RADIO]);
}
