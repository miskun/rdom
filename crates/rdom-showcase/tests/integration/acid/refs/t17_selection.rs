//! Tile 17 — selection, highlights and `user-select`.
//!
//! Spec: CSS UI 4 §6.1 (`user-select: none` content is not selected —
//! a selection across it does not highlight it; `contain` and `all`
//! constrain what a user's selection takes, and a selection made by
//! script that covers them selects them); CSS Pseudo-Elements 4 §3
//! (`::selection` and `::highlight()` style the selected / highlighted
//! text; generated content is not selectable: its used `user-select` is
//! `none`), §3.5 (highlight overlays paint in order, the selection
//! topmost); CSS Custom Highlight API 1 §4–§5 (overlapping highlights by
//! `priority`, then registration order — the higher priority on top
//! whatever the order; each applying only the properties it sets); DOM
//! §5.3 (live ranges: replacing data before a boundary point moves it by
//! the change). DIVERGENCES §2 "A highlight overlay applies the values its
//! pseudo-element changes" (`color`, `background-color` and decorations;
//! a highlight's background paints wherever it covers — `err` sets none,
//! so `hit`'s shows through), "A highlight's ranges are all live".
//!
//! Derivation:
//!
//! - Row 0, `<abNOcd*CTefALLgh>` (`<`, `>` and `*` generated): the
//!   selection runs from after `a` to the end of the paragraph: `b`, `cd`,
//!   `CT` (`contain`), `ef`, `ALL` (`all`), `gh` yellow on navy; `NO`
//!   (`none`), the `*` `::before` and the `>` `::after` not.
//! - Row 1, `ijklmnop`: the selection continues to after `k`. `err`
//!   (registered first, priority 1) covers `lmno`, `hit` (priority 0)
//!   `jklm`. `i`: the selection. `j`, `k`: `hit` under the selection —
//!   the selection's colours. `l`, `m`: `err` over `hit` — `err`'s red text and
//!   cyan underline (its `text-decoration-color`) over `hit`'s green background. `n`, `o`: `err` alone,
//!   on the default background. `p`: nothing.
//! - Row 2: `xyz NEEDLE`, `mv` over `NEEDLE` (4–10); the load script
//!   inserts `++` at 0 (the range 6–12) and replaces `xyz` (2–5) with `w`
//!   (6 − 3 + 1 = 4 to 10): `++w NEEDLE`, `NEEDLE` purple.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "17",
    spec: &[
        "CSS UI 4 §6.1; CSS Pseudo-Elements 4 §3",
        "CSS Custom Highlight API 1 §4–§5; DOM §5.3",
        "DIVERGENCES §2 highlight overlay, live highlight ranges",
    ],
    legend: &[
        ('S', "fg #ffff00 bg #000080"),
        ('E', "fg #ff0000 bg #008000 ul #00ffff underline"),
        ('R', "fg #ff0000 ul #00ffff underline"),
        ('m', "bg #800080"),
    ],
    grid: r#"
|<abNOcd*CTefALLgh>                    |
|..S..SS.SSSSSSSSS.....................|
|ijklmnop                              |
|SSSEERR...............................|
|++w NEEDLE                            |
|....mmmmmm............................|
"#,
};
