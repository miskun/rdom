//! I14 — user validity (tile 42; C11-FORM-STATES).
//!
//! Spec: HTML §4.16.3 — `:user-valid` / `:user-invalid` match a valid /
//! invalid control whose **user validity** is true; HTML §4.10.5.5 /
//! §4.10.18.5: it becomes true when the user commits a change — for a
//! text field, when it loses focus with an edited value, `change` firing
//! before `blur` (§4.10.5.5 "fire a change event … when the element
//! loses focus"); for a checkbox, when its activation fires `change` —
//! and for every control of a form whose submission is attempted
//! (§4.10.21.3 "interactively validate the constraints"); the form's
//! reset sets it false again and restores the default values
//! (§4.10.22). Validity: `pattern` (§4.10.5.3.6, an empty value never
//! mismatches), `required` (a checkbox suffering from being missing while
//! unchecked). A blocked submission focuses the first invalid control
//! (DIVERGENCES §2 "No validation bubble"); script focus during a click is
//! pointer focus, on a checkbox not evident (`FOCUS-VOCAB-1`). Selectors 4
//! §4.5 (`form:has(:user-invalid)`).
//!
//! Derivation, from tile 42's rest state:
//!
//! 1. Click in the pattern field: focused (`#2d2f31`), the block caret
//!    white at x 1.
//! 2. Type `x`: it mismatches `[0-9]+`, but nothing was committed — no
//!    `:user-*` colour: `x` white, the label plain, the log `log:`.
//! 3. Tab: the field commits — `change`, then `blur` (`log:cb`) — and is
//!    `:user-invalid`: `x` red; the checkbox focused by the key, tinted;
//!    the form `:has(:user-invalid)` — `form` red.
//! 4. Click after `x`, Backspace, type `5`, Tab: committed again
//!    (`log:cbcb`), now `:user-valid` — `5` green; the label plain.
//! 5. Click `[ Go ]`: the submission is attempted — every control's user
//!    validity true: the field `:user-valid` green, the unchecked
//!    `required` checkbox `:user-invalid` red, the empty `required` field
//!    too (its placeholder red); the submission is blocked and the first
//!    invalid control, the checkbox, focused without a tint; `form` red.
//! 6. Click `[ R ]`: the form resets — the field empty, user validity
//!    false everywhere: the rest state but for the log.
//! 7. Click the checkbox: checked, committed — `:user-valid`, `[x] `
//!    green.
//! 8. Click it again: unchecked — `:user-invalid`, `[ ] ` red; `form`
//!    red.

use crossterm::event::KeyCode;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I14",
    title: "User validity",
    page: 11,
    spec: &["HTML §4.16.3, §4.10.5.5, §4.10.21.3, §4.10.22; Selectors 4 §4.5"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "HTML §4.16.3, §4.10.5.5, §4.10.21.3, §4.10.22",
    "Selectors 4 §4.5; DIVERGENCES §2 No validation bubble, FOCUS-VOCAB-1",
];

const LEGEND: &[(char, &str)] = &[
    ('f', "bg #1f2123"),
    ('F', "bg #2d2f31"),
    ('w', "fg #ffffff bg #2d2f31"),
    ('W', "bg #ffffff"),
    ('R', "fg #ff0000 bg #1f2123"),
    ('G', "fg #00a000 bg #1f2123"),
    ('r', "fg #ff0000"),
    ('g', "fg #00a000"),
    ('p', "fg #808080 bg #1f2123"),
    ('P', "fg #ff0000 bg #1f2123"),
    ('B', "fg #1e90ff bold"),
];

static CLICKED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|      [ ]   rq   [ Go ] [ R ] form log:         |
|FWFFF......fppff.BBBBBB.BBBBB...................|
"#,
};

static TYPED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| x    [ ]   rq   [ Go ] [ R ] form log:         |
|FwWFF......fppff.BBBBBB.BBBBB...................|
"#,
};

static TABBED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| x    [ ]   rq   [ Go ] [ R ] form log:cb       |
|fRfff.FFFF.fppff.BBBBBB.BBBBB.rrrr..............|
"#,
};

static FIXED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| 5    [ ]   rq   [ Go ] [ R ] form log:cbcb     |
|fGfff.FFFF.fppff.BBBBBB.BBBBB...................|
"#,
};

static SUBMITTED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| 5    [ ]   rq   [ Go ] [ R ] form log:cbcb     |
|fGfff.rrrr.fPPff.BBBBBB.BBBBB.rrrr..............|
"#,
};

static RESET: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|      [ ]   rq   [ Go ] [ R ] form log:cbcb     |
|fffff......fppff.BBBBBB.BBBBB...................|
"#,
};

static CHECKED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|      [x]   rq   [ Go ] [ R ] form log:cbcb     |
|fffff.gggg.fppff.BBBBBB.BBBBB...................|
"#,
};

static UNCHECKED: Reference = Reference {
    tile: "42",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|      [ ]   rq   [ Go ] [ R ] form log:cbcb     |
|fffff.rrrr.fppff.BBBBBB.BBBBB.rrrr..............|
"#,
};

fn run(s: &mut Session) {
    s.click("42", 2, 0);
    s.expect("click in the pattern field", &[&CLICKED]);
    s.type_text("x");
    s.expect("typed a mismatch", &[&TYPED]);
    s.key(KeyCode::Tab);
    s.expect("Tab leaves it", &[&TABBED]);
    s.click("42", 2, 0);
    s.key(KeyCode::Backspace);
    s.type_text("5");
    s.key(KeyCode::Tab);
    s.expect("fixed and left again", &[&FIXED]);
    s.click("42", 19, 0);
    s.expect("the submit button clicked", &[&SUBMITTED]);
    let cb = s.find("42", ".cb");
    let focused = s.dom().focused();
    s.check_eq(
        "the blocked submission focuses the first invalid control",
        focused,
        Some(cb),
        "HTML §4.10.21.3; DIVERGENCES §2 No validation bubble",
    );
    s.click("42", 26, 0);
    s.expect("the reset button clicked", &[&RESET]);
    s.click("42", 7, 0);
    s.expect("the required checkbox checked", &[&CHECKED]);
    s.click("42", 7, 0);
    s.expect("and unchecked", &[&UNCHECKED]);
}
