//! Tile 42 — user validity, at rest (the state step I14 starts from).
//!
//! Spec: HTML §4.16.3 — `:user-valid` / `:user-invalid` match only once
//! the control's user validity is true (after the user commits a value,
//! or a submission is attempted); nothing has been, so neither colours
//! anything, and the form's `:has(:user-invalid)` label stays plain; the
//! `pattern` field is empty, which no pattern constrains. The UA's
//! fields (`#1f2123`, `padding: 0 1`), checkbox `[ ] `, buttons
//! (`[ … ]`, bold accent); the author's grey `::placeholder`. CSS
//! Flexbox 1 §9 (`gap: 1`).
//!
//! Derivation: the pattern field x 0–4, the checkbox x 6–9, the
//! `required` field x 11–15 with `rq` grey at x 12, `[ Go ]` x 17–22,
//! `[ R ]` x 24–28, `form` x 30–33, `log:` x 35.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "42",
    spec: &["HTML §4.16.3, §15.5; CSS Flexbox 1 §9; DIVERGENCES §1 text field"],
    legend: &[
        ('f', "bg #1f2123"),
        ('p', "fg #808080 bg #1f2123"),
        ('B', "fg #1e90ff bold"),
    ],
    grid: r#"
|      [ ]   rq   [ Go ] [ R ] form log:         |
|fffff......fppff.BBBBBB.BBBBB...................|
"#,
};
