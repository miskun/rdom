//! I3 — Tab vs click focus (tile 35).
//!
//! Spec: Selectors 4 §13.2 — `:focus-visible` matches when the UA
//! determines the focus should be made evident; HTML §6.6 leaves the
//! heuristics to the UA, and the browsers' (Selectors 4 §13.2's
//! examples, the WICG polyfill they converged on) are DIVERGENCES §2
//! `FOCUS-VOCAB-1`'s: keyboard focus is evident, a mouse click on a
//! button is not, a text field's focus always is, script focus from a
//! click handler counts as pointer focus. HTML §6.6.2 (Tab moves to the
//! next focusable area in tree order — page 10's first is tile 35's
//! button); CSS UI 4 §5 / DIVERGENCES §1 "An outline is a whole-cell
//! ring" (`outline: auto` with `outline-color: auto`: a light ring with
//! rounded corners in the accent, on the cells just outside the border
//! box); the UA's `button:focus-visible` tint and `input:focus-visible`
//! `!important` background `#2d2f31`; DIVERGENCES §2 "Caret paint" /
//! "`caret-shape`" (`auto` is a block: the caret cell's background the
//! cascaded foreground — the fields are yellow — steady under
//! `App::with_backend`); HTML §6.6.3 / C12-FOCUS-FLUSH (`focus()` on an
//! element the same listener just unhid focuses it at once: the focusing
//! steps update style first).
//!
//! Derivation, from tile 35's rest state:
//!
//! 1. Tab: the button is focused, evidently — the ring `╭──────╮` /
//!    `│ … │` / `╰──────╯` at x 0–7, rows 0–2 (its border box is x 1–6,
//!    row 1), accent; the button tinted `#2d2f31`.
//! 2. Tab: the field is focused — no ring; the field's five cells
//!    `#2d2f31` and the caret block at its content start (x 10) yellow.
//! 3. Click on the button: pointer focus on a button is not evident — no
//!    ring, no tint: the rest state's cells, the button focused.
//! 4. Click on the field: a text field's focus is evident — as 2 (an
//!    empty field puts the caret at its start wherever it is clicked).
//! 5. Click on `show`: its listener unhides the panel and focuses its
//!    field, which holds the focus at once — the log reads `ok` (x
//!    15–16, after the panel, x 9–13); script focus in a click handler
//!    is pointer focus, on a text field evident: the panel's field
//!    `#2d2f31` with its caret at x 10; the first field back to rest.

use super::super::super::reference::Reference;
use super::super::super::refs::t35_focus;
use super::super::session::{Session, Step};
use crossterm::event::KeyCode;

pub static STEP: Step = Step {
    id: "I3",
    title: "Tab vs click focus",
    page: 10,
    spec: &["Selectors 4 §13.2; HTML §6.6; CSS UI 4 §5; DIVERGENCES FOCUS-VOCAB-1"],
    run,
};

const SPEC: &[&str] = &[
    "Selectors 4 §13.2; HTML §6.6.2; CSS UI 4 §5",
    "DIVERGENCES §1 outline ring, §2 FOCUS-VOCAB-1, caret paint",
];

static TAB_BUTTON: Reference = Reference {
    tile: "35",
    spec: SPEC,
    legend: &[
        ('a', "fg #1e90ff"),
        ('T', "fg #1e90ff bg #2d2f31 bold"),
        ('B', "fg #1e90ff bold"),
        ('f', "bg #1f2123"),
    ],
    grid: r#"
|╭──────╮                              |
|aaaaaaaa..............................|
|│[ go ]│                              |
|aTTTTTTa.fffff........................|
|╰──────╯                              |
|aaaaaaaa..............................|
|[ show ]                              |
|BBBBBBBB..............................|
"#,
};

static FIELD_FOCUSED: Reference = Reference {
    tile: "35",
    spec: SPEC,
    legend: &[
        ('B', "fg #1e90ff bold"),
        ('F', "bg #2d2f31"),
        ('c', "bg #ffff00"),
    ],
    grid: r#"
|                                      |
|......................................|
| [ go ]                               |
|.BBBBBB..FcFFF........................|
|                                      |
|......................................|
|[ show ]                              |
|BBBBBBBB..............................|
"#,
};

static PANEL_FOCUSED: Reference = Reference {
    tile: "35",
    spec: SPEC,
    legend: &[
        ('B', "fg #1e90ff bold"),
        ('f', "bg #1f2123"),
        ('F', "bg #2d2f31"),
        ('c', "bg #ffff00"),
    ],
    grid: r#"
|                                      |
|......................................|
| [ go ]                               |
|.BBBBBB..fffff........................|
|                                      |
|......................................|
|[ show ]       ok                     |
|BBBBBBBB.FcFFF........................|
"#,
};

fn run(s: &mut Session) {
    s.key(KeyCode::Tab);
    s.expect("Tab to the button", &[&TAB_BUTTON]);
    s.key(KeyCode::Tab);
    s.expect("Tab to the field", &[&FIELD_FOCUSED]);
    s.click("35", 3, 1);
    let fb = s.find("35", ".fb");
    let focused = s.dom().focused();
    s.check_eq(
        "click on the button: focused",
        focused,
        Some(fb),
        "HTML §6.6",
    );
    s.expect("click on the button", &[&t35_focus::REF]);
    s.click("35", 11, 1);
    s.expect("click on the field", &[&FIELD_FOCUSED]);
    s.click("35", 2, 3);
    let pi = s.find("35", ".pi");
    let focused = s.dom().focused();
    s.check_eq(
        "the shown panel's field focused by the click handler",
        focused,
        Some(pi),
        "HTML §6.6.3; C12-FOCUS-FLUSH",
    );
    s.expect("click on show", &[&PANEL_FOCUSED]);
}
