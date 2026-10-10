//! Tile 22 — container queries and containment.
//!
//! Spec: CSS Conditional 5 §6.1 (`container-type: inline-size` / `size`;
//! `container: <name> / <type>`), §6.2 (`@container <name>` queries the
//! nearest ancestor container of that name; unnamed, the nearest
//! container of a fitting type), §6.4 (a size feature a container does not
//! answer — `height` on an `inline-size` container — is unknown, so the
//! query is false), §6.5 (`style()` queries: every element is a style
//! container; the custom property's computed value), §6.6 (`cqw` is 1 % of
//! the query container's width, `cqmin` of the smaller of its width and
//! height); CSS Containment 2 §3.1 (layout containment: the box is the
//! containing block of its `fixed` descendants), §3.3 (style containment:
//! `counter-increment` inside the subtree is scoped to it and creates a
//! new counter), §3.4 (paint containment clips to the padding box), §4
//! (`content-visibility: hidden` skips the contents and applies size
//! containment: the box keeps its border, its content 0 tall); CSS Will
//! Change 1 §3 (`will-change: opacity` makes the stacking context
//! `opacity` would); CSS 2.1 Appendix E (a context's negative `z-index`
//! children paint after its own background). DIVERGENCES §2 "Container
//! queries are evaluated in layout passes" (the result the browser's),
//! "Stacking contexts form from … a `will-change` naming such a property".
//!
//! Derivation (three flex bands; rows 0, 4, 8):
//!
//! - Row 0: the card's two parts are blocks. The sidebar (x 0, 20 wide)
//!   is under 30: the card stays a block container — `IMG`, then `text`; its bar 50 % of 20 = 10 (row 2). The pane
//!   (x 22, `calc(20vw + 12)`: 36 at 120 columns — CSS Values 4 §6.1.2,
//!   DIVERGENCES §1 "The viewport is the terminal") is 30 or more: the card
//!   is a row, gap 1 — `IMG text`; its bar 18 (row 1). Step I19 narrows
//!   the terminal under it.
//! - Row 4: inside the 10-wide `.inner`, inside the 24-wide container
//!   `box`: `named` queries `box` (24 > 20, green), `near` the nearest
//!   container, `.inner` (10, not). The `size` container (x 26, 10 × 3):
//!   `tall` green (3 > 2); its bar `100cqmin` = min(10, 3) = 3 (navy, row
//!   5). The `inline-size` container (x 38): `(height > 2)` unknown, `unk`
//!   default. `sty` (x 48) under `--theme: dark`: green.
//! - Row 8: `contain: paint` (4 wide) cuts its 8-wide maroon child to
//!   `abcd`; the `contain: layout` box (x 6, 8 × 2, navy) places its
//!   `fixed` child `F` (teal) at its (1, 1); the counters: `a` 1, `b` in
//!   the style-contained span a new counter — 1 —, `c` the outer counter's
//!   2: `1a 1b 2c`; the `content-visibility: hidden` box (x 26): its
//!   border around no content — 6 × 2, `zz` skipped; the `will-change:
//!   opacity` box (x 34, navy) is a stacking context, so its `z-index: -1`
//!   child `N` (yellow) paints above its background.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "22",
    spec: &[
        "CSS Conditional 5 §6.1–§6.6",
        "CSS Containment 2 §3.1, §3.3, §3.4, §4; CSS Will Change 1 §3; CSS 2.1 Appendix E",
        "DIVERGENCES §2 container queries, stacking contexts",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('n', "bg #000080"),
        ('g', "fg #00a000"),
        ('m', "bg #800000"),
        ('y', "fg #ffff00 bg #000080"),
    ],
    grid: r#"
|IMG                   IMG text                            |
|..........................................................|
|text                                                      |
|......................tttttttttttttttttt..................|
|                                                          |
|tttttttttt................................................|
|                                                          |
|..........................................................|
|named near                tall        unk       sty       |
|ggggg.....................gggg..................ggg.......|
|                                                          |
|..........................nnn.............................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|abcd            1a 1b 2c  ┌────┐  N                       |
|mmmm..nnnnnnnn....................ynnn....................|
|       F                  └────┘                          |
|......ntnnnnnn............................................|
"#,
};
