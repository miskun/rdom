//! I9 — caret blink, an edit, caret shapes and the pointer's shape
//! (tile 40; C12-CARET, C12-CURSOR).
//!
//! Spec: HTML §6.6 / §4.10.5.4 (a click in a text field focuses it and
//! places the caret at the clicked boundary — the cell after the text is
//! its end; typing inserts at the caret; Left moves it one character);
//! CSS UI 4 §6.2.1 (`caret-animation: auto` — the UA may blink;
//! `manual` — it must not), §6.2.2 (`caret-shape`), §4.1 (`cursor`; HTML
//! §15.3.4 a link's `pointer`; `auto` over editable text `text`). The
//! terminal mapping: DIVERGENCES §2 "Caret paint" (a block caret: the
//! cell's background `caret-color`, its glyph `caret-text-color`; the
//! blink on for a period, off for one, restarting on every key and
//! click — 500 ms here, `App::with_caret_blink`), "`caret-shape` draws
//! the painted caret in whole cells" (`underscore`: the glyph kept, the
//! cell underlined in `caret-color`; `bar`: `▏` in `caret-color` where
//! the cell shows no glyph, the underscore's underline over a glyph),
//! "`cursor` is the terminal pointer's shape" (OSC 22 with the CSS name,
//! to a terminal known to take it — `PointerShapes::Osc22` here); the
//! UA's `input:focus-visible` `#2d2f31`, a clicked text field's focus
//! being evident (`FOCUS-VOCAB-1`).
//!
//! Derivation (blink period 500 ms; every click at clock 0 or after an
//! advance, so each phase is read inside it):
//!
//! 1. Click after `ab` in the first field (x 3): focused (`#2d2f31`), the
//!    caret block at x 3 — yellow.
//! 2. +600 ms: the off phase — no caret.
//! 3. Type `c`: `abc`, the caret after it (x 4), on again (an edit
//!    restarts the blink).
//! 4. Click after `ab` in the second field (x 10): the first unfocused
//!    (`abc` on the field background); the underscore caret on the blank
//!    end cell — an underline in red.
//! 5. Left: the caret before `b` — `b` underlined red, the end cell plain.
//! 6. Click after `ab` in the third field (x 17): `▏` green at x 17.
//! 7. +600 ms: still `▏` — `manual` does not blink.
//! 8. Left: over `b` (x 16) the bar is an underline in green.
//! 9. The pointer, from the third field: over the link `pointer`, over
//!    the `grab` box `grab`, back over the field `text` — each an OSC 22
//!    (a change is sent once, so the hovers alternate). The underscore on
//!    the blank end cell (4) is drawn in its own underline colour, so the
//!    blank's foreground is not seen there (the reference format's rule).

use std::time::Duration;

use crossterm::event::KeyCode;
use rdom_tui::PointerShapes;
use rdom_tui::render::TestBackend;
use rdom_tui::runtime::app::App;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I9",
    title: "Caret blink, an edit, caret shapes, pointer",
    page: 10,
    spec: &["HTML §6.6; CSS UI 4 §4.1, §6.2; DIVERGENCES §2 caret, cursor"],
    run,
    configure: Some(configure),
};

fn configure(app: App<TestBackend>) -> App<TestBackend> {
    app.with_caret_blink(Some(Duration::from_millis(500)))
        .with_pointer_shapes(PointerShapes::Osc22)
}

const SPEC: &[&str] = &[
    "HTML §6.6; CSS UI 4 §6.2.1, §6.2.2",
    "DIVERGENCES §2 caret paint, caret-shape; FOCUS-VOCAB-1",
];

const LEGEND: &[(char, &str)] = &[
    ('f', "bg #1f2123"),
    ('w', "fg #ffffff bg #1f2123"),
    ('F', "bg #2d2f31"),
    ('X', "fg #ffffff bg #2d2f31"),
    ('y', "bg #ffff00"),
    ('u', "fg #ffffff bg #2d2f31 ul #ff0000 underline"),
    ('U', "bg #2d2f31 ul #ff0000 underline"),
    ('G', "fg #00a000 bg #2d2f31"),
    ('v', "fg #ffffff bg #2d2f31 ul #00a000 underline"),
    ('L', "fg #1e90ff underline"),
];

static F1_ON: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| ab     ab     ab                     |
|FXXyFF.fwwfff.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F1_OFF: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| ab     ab     ab                     |
|FXXFFF.fwwfff.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F1_TYPED: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| abc    ab     ab                     |
|FXXXyF.fwwfff.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F2_END: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| abc    ab     ab                     |
|fwwwff.FXXUFF.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F2_B: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| abc    ab     ab                     |
|fwwwff.FXuFFF.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F3_END: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| abc    ab     ab▏                    |
|fwwwff.fwwfff.FXXGFF..................|
|link grab                             |
|LLLL..................................|
"#,
};

static F3_B: Reference = Reference {
    tile: "40",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
| abc    ab     ab                     |
|fwwwff.fwwfff.FXvFFF..................|
|link grab                             |
|LLLL..................................|
"#,
};

fn run(s: &mut Session) {
    s.click("40", 3, 0);
    s.expect("click after ab in the first field", &[&F1_ON]);
    s.advance(600);
    s.expect("600 ms: the off phase", &[&F1_OFF]);
    s.type_text("c");
    s.expect("typed c", &[&F1_TYPED]);
    s.click("40", 10, 0);
    s.expect("click after ab in the underscore field", &[&F2_END]);
    s.key(KeyCode::Left);
    s.expect("Left: before b", &[&F2_B]);
    s.click("40", 17, 0);
    s.expect("click after ab in the bar field", &[&F3_END]);
    s.advance(600);
    s.expect("600 ms: manual, still on", &[&F3_END]);
    s.key(KeyCode::Left);
    s.expect("Left: the bar over b", &[&F3_B]);
    // From the third field, where the last click left the pointer: each
    // hover a change of shape.
    for (dx, dy, shape) in [(1, 1, "pointer"), (6, 1, "grab"), (15, 0, "text")] {
        s.hover("40", dx, dy);
        let out = s.output();
        let osc = format!("\x1b]22;{shape}\x1b\\");
        s.check(
            &format!("the pointer's shape over ({dx}, {dy})"),
            out.contains(&osc),
            format!("{out:?}"),
            "CSS UI 4 §4.1; DIVERGENCES §2 cursor",
        );
    }
}
