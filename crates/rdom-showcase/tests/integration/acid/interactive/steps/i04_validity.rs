//! I4 — type into the `required` field (tile 36, row 0).
//!
//! Spec: HTML §4.10.5.1 / §4.10.21.1 — a `required` text field whose
//! value is empty suffers from being missing; with a value it does not,
//! so it moves from `:invalid` to `:valid` as the user types, and back
//! when the value is deleted; Selectors 4 §14.4 / HTML §4.16.3 — a form
//! is `:valid` when none of its submittable elements is invalid, so the
//! form follows. HTML §6.6 / DIVERGENCES `FOCUS-VOCAB-1` (a clicked text
//! field's focus is evident: `#2d2f31`, `!important` over the field
//! background); DIVERGENCES §2 "Caret paint" (a block caret, its
//! background the field's white text colour; steady).
//!
//! Derivation, from tile 36's rest state:
//!
//! 1. Click in the field: focused, the five cells `#2d2f31`, the caret
//!    at the content start (x 1); still `bad` / `form` red.
//! 2. Type `x`: `x` at x 1 in white, the caret after it (x 2); the field
//!    valid — `ok` green (x 6–7), the form valid — `form` green, one cell
//!    left of where it was (`ok` is a cell narrower than `bad`, x 9–12).
//! 3. Backspace: empty again — invalid, `bad` and `form` red at their
//!    rest cells, the caret back at x 1.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};
use crossterm::event::KeyCode;

pub static STEP: Step = Step {
    id: "I4",
    title: "Type into the required field",
    page: 10,
    spec: &["HTML §4.10.5.1, §4.10.21.1, §4.16.3; Selectors 4 §14.4"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "HTML §4.10.21.1, §4.16.3; Selectors 4 §14.4",
    "DIVERGENCES §2 FOCUS-VOCAB-1, caret paint",
];

static FOCUSED: Reference = Reference {
    tile: "36",
    spec: SPEC,
    legend: &[
        ('F', "bg #2d2f31"),
        ('w', "bg #ffffff"),
        ('r', "fg #ff0000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|      bad form                        |
|FwFFF.rrr.rrrr........................|
|[ ] (•) ( ) (•)                       |
|....gggg....gggg......................|
"#,
};

static TYPED: Reference = Reference {
    tile: "36",
    spec: SPEC,
    legend: &[
        ('F', "bg #2d2f31"),
        ('X', "fg #ffffff bg #2d2f31"),
        ('w', "bg #ffffff"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
| x    ok form                         |
|FXwFF.gg.gggg.........................|
|[ ] (•) ( ) (•)                       |
|....gggg....gggg......................|
"#,
};

fn run(s: &mut Session) {
    s.click("36", 2, 0);
    s.expect("click in the field", &[&FOCUSED]);
    s.type_text("x");
    s.expect("typed x", &[&TYPED]);
    s.key(KeyCode::Backspace);
    s.expect("deleted it", &[&FOCUSED]);
}
