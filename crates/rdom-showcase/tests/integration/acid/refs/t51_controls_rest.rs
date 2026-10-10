//! Tile 51 — a range slider, a pixel `outline-offset`, an unchecked
//! default box (tile 15a's left-out cases).
//!
//! Spec: HTML §4.10.5.1.13 (a range control, its value 5 of 0–10; its
//! look the UA's: rdom's slider is a track `─` across the box with the
//! thumb `●` at `round(ratio · (width − 1))` — `runtime::builtins::range`
//! — in the accent `#1e90ff`, DIVERGENCES §2 `accent-color`); CSS UI 4 §5
//! (`outline-offset` moves the ring out; DIVERGENCES §1 "An outline is a
//! whole-cell ring": a pixel offset is one cell its way); HTML §4.16.3
//! (`:default` matches a checkbox with a `checked` attribute whatever its
//! checkedness; `:checked` follows the checkedness the load script set
//! false — `[ ] `, yellow); CSS Flexbox 1 §9 (`gap: 4`).
//!
//! Derivation (row 2, `padding: 2 0`): the 11-wide slider at x 0–10, the
//! thumb at x 5; `ab` at x 15, its ring two cells out — x 13–18, rows
//! 0–4; the checkbox at x 21.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "51",
    spec: &[
        "HTML §4.10.5.1.13, §4.16.3; CSS UI 4 §5; CSS Flexbox 1 §9",
        "DIVERGENCES §1 outline ring; §2 accent-color",
    ],
    legend: &[('a', "fg #1e90ff"), ('y', "fg #ffff00")],
    grid: r#"
|             ┌────┐                   |
|......................................|
|             │    │                   |
|......................................|
|─────●─────  │ ab │  [ ]              |
|aaaaaaaaaaa..........yyyy.............|
|             │    │                   |
|......................................|
|             └────┘                   |
|......................................|
"#,
};
