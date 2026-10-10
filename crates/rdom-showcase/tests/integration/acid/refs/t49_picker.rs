//! Tile 49 — a picker and an article by the terminal's height, at rest
//! (the state step I21 starts from).
//!
//! Spec: CSS Values 4 §6.1.2 / DIVERGENCES §1 "The viewport is the
//! terminal" (`100vh` of 50 rows: the article 26 wide); CSS Multi-column 1
//! §3.4 (`column-width: 8`, `column-gap: 1`: ⌊27 / 9⌋ = 3 columns of
//! (26 − 2) / 3 = 8, at x 0, 9, 18), §7 (balanced: six one-row lines in 2
//! rows); HTML §6.12 (the popover closed); the UA's button.
//!
//! Derivation: rows 0–1 `c1`/`c2`, `c3`/`c4`, `c5`/`c6`; `[ pick ]` at
//! x 30, row 5.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "49",
    spec: &[
        "CSS Values 4 §6.1.2; CSS Multi-column 1 §3.4, §7; HTML §6.12",
        "DIVERGENCES §1 viewport; §2 multi-column layout",
    ],
    legend: &[('B', "fg #1e90ff bold")],
    grid: r#"
|c1       c3       c5                                      |
|..........................................................|
|c2       c4       c6                                      |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                              [ pick ]                    |
|..............................BBBBBBBB....................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};
