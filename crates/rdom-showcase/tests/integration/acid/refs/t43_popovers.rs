//! Tile 43 — popover light dismiss, at rest (the state step I15 starts
//! from).
//!
//! Spec: HTML §6.12 (a `[popover]` not showing is `display: none`),
//! §4.11.4 (a closed `<dialog>` likewise); the UA's buttons (`[ … ]`,
//! bold accent); CSS Position 3 §3 (`.out` absolute at row 9).
//!
//! Derivation: row 0 `[ open ]` (x 0–7) and `[ man ]` (x 9–15); row 9
//! `outside`; nothing else.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "43",
    spec: &["HTML §6.12, §4.11.4, §15.5; CSS Position 3 §3"],
    legend: &[('B', "fg #1e90ff bold")],
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
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|outside                                                   |
|..........................................................|
"#,
};
