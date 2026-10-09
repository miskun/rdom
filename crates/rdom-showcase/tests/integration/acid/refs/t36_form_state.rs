//! Tile 36 — form state, at rest (the state steps I4 and I5 start from).
//!
//! Spec: HTML §4.10.5.1 / §4.16.3 — a `required` empty text field
//! suffers from being missing, so it is `:invalid`, and a form with an
//! invalid submittable element is `:invalid` (Selectors 4 §14.4); HTML
//! §4.10.5.1.15 — a radio button group is the radios of one name with the
//! same form owner (the one outside the form has none), so the two
//! checked radios are in different groups and both stay checked; the
//! toggles' marks as tile 15a's reference cites them (`[ ] `, `( ) `,
//! `(•) `, 4-wide inline blocks); CSS Flexbox 1 §9 (each row a flex row,
//! row 0 with `gap: 1`).
//!
//! Derivation: row 0 the field (5 cells, `width: 3` plus its padding) on
//! the field background, `bad` red (x 6–8), `form` red (x 10–13); row 1
//! the checkbox `[ ] `, the checked radio `(•) ` green, `( ) `, and the
//! outside radio `(•) ` green.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "36",
    spec: &[
        "HTML §4.10.5.1, §4.10.5.1.15, §4.16.3; Selectors 4 §14.4",
        "CSS Flexbox 1 §9; DIVERGENCES §1 text field",
    ],
    legend: &[
        ('f', "bg #1f2123"),
        ('r', "fg #ff0000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|      bad form                        |
|fffff.rrr.rrrr........................|
|[ ] (•) ( ) (•)                       |
|....gggg....gggg......................|
"#,
};
