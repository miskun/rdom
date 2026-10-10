//! I21 — scroll an anchor and resize past a picker (tiles 26 and 49;
//! C15-ANCHOR, C15-COLUMNS).
//!
//! Spec: CSS Anchor Positioning 1 §5 (`position-visibility:
//! anchors-visible` shows the box only while its anchor is visible — not
//! clipped out of its scroll container; `always` shows it wherever its
//! anchor is), §3.2 (`anchor()` follows the anchor as it scrolls, every
//! frame), §3.1 (`position-area: bottom span-right`: below the anchor,
//! from its left edge), §4 (`position-try-fallbacks: flip-block`: when
//! the box overflows its inset-modified containing block — for a
//! `position: fixed` popover, the viewport — the option with the block
//! axis flipped, above the anchor; the options are tried again at every
//! layout, so a taller viewport puts it back below); HTML §6.12 (the
//! `popovertarget` button shows it); CSS Multi-column 1 §3.4 / §7 (with
//! `column-width` the count follows the width; balanced sets); CSS
//! Values 4 §6.1.2 (`vh`). DIVERGENCES §2 "Anchor positioning is laid out
//! in whole cells", "Multi-column layout is laid out in whole cells" (the
//! column width floored), §1 "The viewport is the terminal". `ACID.md`
//! puts the resized article in tile 25; its static article has no height
//! to follow, so the article that does is tile 49's, beside the picker.
//! That the resize lays the article out once is not visible in cells.
//!
//! Derivation:
//!
//! 1. Wheel ticks over tile 26's scroller (its anchor `S` 4 rows down in
//!    a 2-row port, range 3): offset 1 — `S` at row 8, still below the
//!    port: `v` hidden, `V` (always) beside it at row 8; offset 2 — row
//!    7; offset 3 — `S` on the port's last row 6: `S`, `v` left of it at
//!    x 19, `V` at x 26. A tick back: offset 2 — `S` out again, `v` gone,
//!    `V` at row 7.
//! 2. The terminal 120 × 46: tile 49's article `100vh − 24` = 22 wide —
//!    ⌊23 / 9⌋ = 2 columns of ⌊21 / 2⌋ = 10 (x 0, 11), the six lines
//!    balanced in 3 rows. `[ pick ]` clicked: the popover (7 × 4, its UA
//!    border) below the button would end at page row 46, past the 46-row
//!    viewport: flipped above — tile rows 1–4, from the button's left
//!    edge (x 30).
//! 3. Back to 120 × 50: the article 26 wide, 3 columns in 2 rows; the
//!    popover fits below again — tile rows 6–8 (its bottom border on page
//!    row 46, under the tile).

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I21",
    title: "Scroll an anchor, resize past a picker",
    page: 7,
    spec: &["CSS Anchor Positioning 1 §3–§5; CSS Multi-column 1 §3.4, §7; HTML §6.12"],
    run,
    configure: None,
};

const SPEC_26: &[&str] = &["CSS Anchor Positioning 1 §3.2, §5; DIVERGENCES §2 anchor positioning"];
const SPEC_49: &[&str] = &[
    "CSS Anchor Positioning 1 §3.1, §4; HTML §6.12; CSS Multi-column 1 §3.4, §7; CSS Values 4 §6.1.2",
    "DIVERGENCES §1 viewport; §2 anchor positioning, multi-column layout",
];

static OFFSET_1: Reference = Reference {
    tile: "26",
    spec: SPEC_26,
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|                                                          |
|..........................................................|
|  A                                         P4    D       |
|..nnnn......................................mmmmmm........|
|      P1       tip                                        |
|......tttt.....ooo........................................|
|              button                                      |
|..........................................................|
|                                                          |
|..........................................................|
|  x1    y2                                                |
|...g.....g................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                          V                               |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};

static OFFSET_2: Reference = Reference {
    tile: "26",
    spec: SPEC_26,
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|                                                          |
|..........................................................|
|  A                                         P4    D       |
|..nnnn......................................mmmmmm........|
|      P1       tip                                        |
|......tttt.....ooo........................................|
|              button                                      |
|..........................................................|
|                                                          |
|..........................................................|
|  x1    y2                                                |
|...g.....g................................................|
|                                                          |
|..........................................................|
|                          V                               |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};

static OFFSET_3: Reference = Reference {
    tile: "26",
    spec: SPEC_26,
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|                                                          |
|..........................................................|
|  A                                         P4    D       |
|..nnnn......................................mmmmmm........|
|      P1       tip                                        |
|......tttt.....ooo........................................|
|              button                                      |
|..........................................................|
|                                                          |
|..........................................................|
|  x1    y2                                                |
|...g.....g................................................|
|                   vS     V                               |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
"#,
};

static SHORT: Reference = Reference {
    tile: "49",
    spec: SPEC_49,
    legend: &[('B', "fg #1e90ff bold")],
    grid: r#"
|c1         c4                                             |
|..........................................................|
|c2         c5                 ┌─────┐                     |
|..........................................................|
|c3         c6                 │alpha│                     |
|..........................................................|
|                              │beta │                     |
|..........................................................|
|                              └─────┘                     |
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

static TALL: Reference = Reference {
    tile: "49",
    spec: SPEC_49,
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
|                              ┌─────┐                     |
|..........................................................|
|                              │alpha│                     |
|..........................................................|
|                              │beta │                     |
|..........................................................|
"#,
};

fn run(s: &mut Session) {
    s.wheel("26", 21, 5, true);
    s.expect("offset 1", &[&OFFSET_1]);
    s.wheel("26", 21, 5, true);
    s.expect("offset 2", &[&OFFSET_2]);
    s.wheel("26", 21, 5, true);
    s.expect("offset 3: the anchor in view", &[&OFFSET_3]);
    s.wheel("26", 21, 5, false);
    s.expect("back to offset 2", &[&OFFSET_2]);
    s.resize(120, 46);
    s.click("49", 32, 5);
    s.expect("46 rows: the picker opened above", &[&SHORT]);
    s.resize(120, 50);
    s.expect("50 rows: back below", &[&TALL]);
}
