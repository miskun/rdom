//! Tile 15b — the top layer, without a modal.
//!
//! Spec: HTML §4.10.7 and its select picker (an open drop-down's options
//! overlay the page, HTML's `::picker(select)` a popover in the top
//! layer), §6.4 (popovers: `showPopover()` puts the element in the top
//! layer; a `[popover]` not showing is `display: none`) and the rendering
//! section's `[popover]` rules (`position: fixed; inset: 0; margin: auto`,
//! fit-content, `border: solid`, `background-color: Canvas; color:
//! CanvasText`); CSS Position 4 §3 (the top layer paints above every
//! stacking context and outside every clip); CSS Backgrounds 3 §3.2 (a
//! background fills the whole box, padding included). DIVERGENCES §2 "An
//! open drop-down `<select>` lists its options from its own row, in the
//! top layer" (the first option on the field's row, the box staying in
//! flow), §2 "The system colors are the terminal's" (`Canvas` paints the
//! terminal's default background, covering what is beneath), §1
//! "Alignment free space is shared in whole cells" (`auto` margins
//! centring a positioned box round the leading space down); the UA select
//! (`#1f2123` field, white text, options `padding: 0 1`, the selected one
//! dodgerblue `#1e90ff` with black text).
//!
//! Derivation (the tile at page x 30, row 14):
//!
//! - The page text: 22 rows of `0123456789` × 6, from the tile's corner.
//! - `.sh`, `overflow: hidden`, 12 × 1 at (2, 1), holds a 12-wide select
//!   (border-box, `padding: 0 1`) the load script opened: its box stays in
//!   flow on row 1 (the field, x 2–13), its options in its content box (x
//!   3–12) from its own row down — `One` on row 1, `Two` (selected) on row
//!   2, `Three` on row 3 — text from x 4; rows 2 and 3 overflow `.sh`'s
//!   clip and show anyway, over the page text, which keeps x 2 and x 13
//!   and is not moved.
//! - The popover `Popover` with `padding: 0 1`: 7 + 2 + 2 = 11 wide, 3
//!   tall; centred in the 120 × 50 viewport — x (120 − 11) / 2 = 54
//!   (rounded down), row (50 − 3) / 2 = 23 — tile x 24–34, rows 9–11. Its
//!   border in `CanvasText` (the default colour) and its box blanked by
//!   `Canvas`: the padding cells beside `Popover` show no digit. Its
//!   `[popover]` sibling is not showing: nothing. It is `:popover-open`
//!   (Selectors 4 §11): its text green, and its border, `currentcolor`,
//!   with it (ACID-COVERAGE).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "15b",
    spec: &[
        "HTML §4.10.7, §6.4, rendering §15 [popover]",
        "CSS Position 4 §3; CSS Backgrounds 3 §3.2",
        "DIVERGENCES §2 select picker, system colors; §1 alignment",
    ],
    legend: &[
        ('G', "fg #00a000"),
        ('f', "bg #1f2123"),
        ('w', "fg #ffffff bg #1f2123"),
        ('a', "bg #1e90ff"),
        ('b', "fg #000000 bg #1e90ff"),
    ],
    grid: r#"
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|01  One       4567890123456789012345678901234567890123456789|
|..ffwwwfffffff..............................................|
|012 Two      34567890123456789012345678901234567890123456789|
|...abbbaaaaaa...............................................|
|012 Three    34567890123456789012345678901234567890123456789|
|...fwwwwwffff...............................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123┌─────────┐5678901234567890123456789|
|........................GGGGGGGGGGG.........................|
|012345678901234567890123│ Popover │5678901234567890123456789|
|........................GGGGGGGGGGG.........................|
|012345678901234567890123└─────────┘5678901234567890123456789|
|........................GGGGGGGGGGG.........................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
|012345678901234567890123456789012345678901234567890123456789|
|............................................................|
"#,
};
