//! Tile 40 — the caret and the pointer, at rest (the state step I9
//! starts from).
//!
//! Spec: HTML §15.5 / DIVERGENCES §1 text field (`padding: 0 1` on the
//! field background `#1f2123`, content box: `width: 4` is 6 cells); CSS
//! Flexbox 1 §9 (`gap: 1`); the UA's `a[href]` (the accent `#1e90ff`,
//! underlined, HTML §15.3.4). Nothing focused: no caret.
//!
//! Derivation: the fields at x 0–5, 7–12, 14–19, each `ab` in white at
//! its content start; row 1 `link` (x 0–3) and `grab` (x 5–8).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "40",
    spec: &[
        "HTML §15.3.4, §15.5; CSS Flexbox 1 §9",
        "DIVERGENCES §1 text field",
    ],
    legend: &[
        ('f', "bg #1f2123"),
        ('w', "fg #ffffff bg #1f2123"),
        ('L', "fg #1e90ff underline"),
    ],
    grid: r#"
| ab     ab     ab                     |
|fwwfff.fwwfff.fwwfff..................|
|link grab                             |
|LLLL..................................|
"#,
};
