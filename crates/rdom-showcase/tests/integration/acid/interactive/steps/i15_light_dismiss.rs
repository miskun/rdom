//! I15 — popover light dismiss and close requests (tile 43;
//! C11-MODAL-POPOVER).
//!
//! Spec: HTML §6.12 `popover`: `popovertarget` toggles an auto popover;
//! showing runs the popover focusing steps (its `[autofocus]` field is
//! focused) and remembers the previously focused element; a popover
//! shown from an invoker inside an open one is nested — the outer stays
//! open ("topmost popover ancestor"). "Light dismiss open popovers": a
//! pointerdown records the topmost clicked popover; on pointerup, only
//! when it is the same, every auto popover above it is hidden — a click
//! inside the outer popover hides the nested one alone, a press inside
//! and release outside hides nothing, a click outside hides them all, as
//! "hide all popovers until … false" — without returning focus. A close
//! request (Esc) hides the topmost auto popover — focus returning to its
//! previously focused element — one per press, and with no popover left,
//! cancels the topmost modal dialog (§4.11.4: `cancel`, then close;
//! the dialog below stays). A manual popover is touched by neither.
//! Focus moved by the Esc key is evident: the UA's
//! `button:focus-visible` tint (`FOCUS-VOCAB-1`); a text field focused
//! during a click is too.
//!
//! The step text in `ACID.md` once had the click outside return focus to
//! the button; HTML's light dismiss passes `focusPreviousElement` false,
//! so only the close request returns it (corrected with this step).
//!
//! Derivation, from tile 43's rest state:
//!
//! 1. Click `[ man ]`: the manual popover at rows 6–8.
//! 2. Click `[ open ]`: the auto popover at rows 2–4 (border, field,
//!    `[ n ]`), its field focused (tinted, the caret white at x 2).
//! 3. Click `[ n ]`: the nested popover at x 15–21; the field unfocused.
//! 4. Click inside the outer popover (its gap cell, x 6): the nested one
//!    hidden, the outer kept.
//! 5. Press there, release on `outside`: nothing hidden.
//! 6. Click `outside`: the outer popover hidden; the manual one stays;
//!    focus not returned to `[ open ]`.
//! 7. Click `[ open ]`, then `[ n ]` again: both open.
//! 8. Esc: the nested one hidden, focus back on `[ n ]` — tinted.
//! 9. Esc: the outer one hidden, focus back on `[ open ]` — tinted.
//! 10. Two modal dialogs opened, the second above the first: Esc closes
//!     the second only; Esc closes the first. The manual popover is still
//!     open, `[ open ]` focused again: as 9.

use crossterm::event::KeyCode;
use rdom_tui::runtime::builtins::dialog;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I15",
    title: "Popover light dismiss",
    page: 12,
    spec: &["HTML §6.12 popover light dismiss, close requests; §4.11.4"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "HTML §6.12 (popover focusing steps, light dismiss, close requests), §4.11.4",
    "DIVERGENCES §2 FOCUS-VOCAB-1",
];

const LEGEND: &[(char, &str)] = &[
    ('B', "fg #1e90ff bold"),
    ('T', "fg #1e90ff bg #2d2f31 bold"),
    ('f', "bg #1f2123"),
    ('F', "bg #2d2f31"),
    ('W', "bg #ffffff"),
];

static MANUAL: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|BBBBBBBB.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

static OPEN: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|BBBBBBBB.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|┌───────────┐                                             |
|..........................................................|
|│      [ n ]│                                             |
|.FWFFF.BBBBB..............................................|
|└───────────┘                                             |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

static NESTED: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|BBBBBBBB.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|┌───────────┐  ┌─────┐                                    |
|..........................................................|
|│      [ n ]│  │inner│                                    |
|.fffff.BBBBB..............................................|
|└───────────┘  └─────┘                                    |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

static INNER_GONE: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|BBBBBBBB.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|┌───────────┐                                             |
|..........................................................|
|│      [ n ]│                                             |
|.fffff.BBBBB..............................................|
|└───────────┘                                             |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

static ESC_NESTED: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|BBBBBBBB.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|┌───────────┐                                             |
|..........................................................|
|│      [ n ]│                                             |
|.fffff.TTTTT..............................................|
|└───────────┘                                             |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

static ESC_OUTER: Reference = Reference {
    tile: "43",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|[ open ] [ man ]                                          |
|TTTTTTTT.BBBBBBB..........................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|┌──────┐                                                  |
|..........................................................|
|│manual│                                                  |
|..........................................................|
|└──────┘                                                  |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};

fn run(s: &mut Session) {
    let open = |s: &Session, sel: &str| {
        let id = s.find("43", sel);
        s.dom().node(id).matches(":popover-open")
    };
    let dialog_open = |s: &Session, sel: &str| {
        let id = s.find("43", sel);
        s.dom().get_attribute(id, "open").is_some()
    };
    s.click("43", 11, 0);
    s.expect("the manual popover shown", &[&MANUAL]);
    s.click("43", 3, 0);
    s.expect("the auto popover shown", &[&OPEN]);
    let af = s.find("43", ".af");
    let focused = s.dom().focused();
    s.check_eq(
        "its [autofocus] field focused",
        focused,
        Some(af),
        "HTML §6.12 popover focusing steps",
    );
    s.click("43", 9, 3);
    s.expect("the nested popover shown", &[&NESTED]);
    s.click("43", 6, 3);
    s.expect("a click inside the outer one", &[&INNER_GONE]);
    s.press("43", 6, 3);
    s.release("43", 3, 9);
    s.expect("a press inside, a release outside", &[&INNER_GONE]);
    s.click("43", 3, 9);
    s.expect("a click outside", &[&MANUAL]);
    let ob = s.find("43", ".ob");
    let focused = s.dom().focused();
    s.check(
        "light dismiss returns no focus",
        focused != Some(ob),
        format!("{focused:?}"),
        "HTML §6.12 light dismiss: hide all popovers until …, focusPreviousElement false",
    );
    s.click("43", 3, 0);
    s.click("43", 9, 3);
    s.expect("both shown again", &[&NESTED]);
    s.key(KeyCode::Esc);
    s.expect("Esc: the nested one", &[&ESC_NESTED]);
    s.key(KeyCode::Esc);
    s.expect("Esc: the outer one", &[&ESC_OUTER]);
    let (d1, d2) = (s.find("43", ".d1"), s.find("43", ".d2"));
    s.script(|dom| {
        dialog::show_modal(dom, d1).unwrap();
        dialog::show_modal(dom, d2).unwrap();
    });
    s.key(KeyCode::Esc);
    let both = (dialog_open(s, ".d1"), dialog_open(s, ".d2"));
    s.check_eq(
        "Esc cancels the upper modal dialog",
        both,
        (true, false),
        "HTML §4.11.4, §6.12",
    );
    s.key(KeyCode::Esc);
    let both = (dialog_open(s, ".d1"), dialog_open(s, ".d2"));
    s.check_eq(
        "Esc cancels the lower one",
        both,
        (false, false),
        "HTML §4.11.4",
    );
    let manual = open(s, ".pm");
    s.check_eq(
        "the manual popover outlived it all",
        manual,
        true,
        "HTML §6.12",
    );
    s.expect("dialogs closed, focus back on the button", &[&ESC_OUTER]);
}
