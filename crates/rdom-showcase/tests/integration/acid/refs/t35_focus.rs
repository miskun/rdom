//! Tile 35 — focus by keyboard and by pointer, at rest (the state step
//! I3 starts from).
//!
//! Spec: CSS Flexbox 1 §9 (row 1 a flex row with `padding: 1` and
//! `gap: 2`; row 3 one with `gap: 1`); HTML §4.10.6 / §15.5 (the button
//! and the field — the UA's: `[ label ]` bold in the accent `#1e90ff`, a
//! text field `padding: 0 1` on the field background `#1f2123`, content
//! box, so `width: 3` is 5 cells); HTML §4.3 / CSS Display 3 §2.6 (the
//! `hidden` panel is `display: none` — no box, so the empty log is the
//! row's next item); nothing focused, so no ring and no caret.
//!
//! Derivation: row 0 the `.fr` padding; row 1 `[ go ]` at x 1–6, the
//! field at x 9–13; row 2 padding; row 3 `[ show ]` at x 0–7, the log
//! empty.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "35",
    spec: &[
        "CSS Flexbox 1 §9; HTML §4.10.6, §15.5; CSS Display 3 §2.6",
        "DIVERGENCES §1 text field",
    ],
    legend: &[('B', "fg #1e90ff bold"), ('f', "bg #1f2123")],
    grid: r#"
|                                      |
|......................................|
| [ go ]                               |
|.BBBBBB..fffff........................|
|                                      |
|......................................|
|[ show ]                              |
|BBBBBBBB..............................|
"#,
};
