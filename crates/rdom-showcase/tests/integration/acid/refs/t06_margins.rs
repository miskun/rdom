//! Tile 6 — margin collapsing.
//!
//! Spec: CSS 2.1 §8.3.1 (adjoining vertical margins collapse: siblings, a
//! box's top margin and its first in-flow child's, its bottom margin and
//! its last child's when its height is `auto`, an empty box's own margins;
//! padding, a border or a line box between them prevents it; the result is
//! the largest positive margin plus the most negative one; the margins of a
//! box that establishes a new block formatting context do not collapse with
//! its children's); CSS Flexbox 1 §4.2 (flex items' margins never
//! collapse); CSS 2.1 §14.2 (the background covers the padding box);
//! DIVERGENCES §2 "Vertical margins collapse in block flow per CSS 2.1
//! §8.3.1".
//!
//! Derivation. The tile is a flex row of nine 3-cell lanes, a cell apart
//! (lane *i* at x 4*i*). A lane is a flex item, so it is a block
//! formatting context root: margins collapse inside it but none escapes
//! it, and each lane's content starts at row 0. Every `p` is one navy row
//! across its lane; `.par` is teal.
//!
//! - L1 (x 0): `A` with `margin-bottom: 2`, `B` with `margin-top: 1`:
//!   siblings collapse to max(2, 1) = 2 — A row 0, B row 3.
//! - L2 (x 4): `.par` holds `C` with `margin-top: 2`; `.par` has no
//!   padding or border, so C's margin collapses with `.par`'s (0) and the
//!   result is outside `.par`: `.par`'s box starts at row 2, all of it
//!   under C (no teal shows). `D` follows at row 3.
//! - L3 (x 8): `E` (bottom 1), an empty block (top 3, bottom 1), `F`
//!   (top 1): all four margins adjoin through the empty block: max = 3 — E
//!   row 0, F row 4.
//! - L4 (x 12): `E` bottom 3, `F` top −1: 3 + (−1) = 2 — F row 3.
//! - L5 (x 16): `.par` with `padding-top: 1` holds `C` (top 1): the padding
//!   separates them, so no collapse: `.par` teal on rows 0–2 (padding,
//!   C's margin, C), C navy on row 2.
//! - L6 (x 20): a block with only a top border (`───`, the default colour)
//!   holds `C` (top 1): the border separates them: C on row 2.
//! - L7 (x 24): a block whose first line is `x` holds `C` (top 1) after
//!   it: the line box is in between, so C's margin is between siblings
//!   (the line's anonymous block and C): C on row 2.
//! - L8 (x 28): a column flex container: `G` (bottom 1) and `H` (top 1)
//!   are flex items, whose margins add: H on row 0 + 1 + 2 = 3.
//! - L9 (x 32): `.par` holds `J` (bottom 2): J's bottom margin collapses
//!   with `.par`'s (an `auto` height, no padding or border), so `.par` is
//!   J's one row and `K` follows at row 1 + 2 = 3; no teal shows.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "6",
    spec: &[
        "CSS 2.1 §8.3.1, §14.2",
        "CSS Flexbox 1 §4.2",
        "DIVERGENCES §2 vertical margins collapse",
    ],
    legend: &[('n', "bg #000080"), ('t', "bg #008080")],
    grid: r#"
|A       E   E       ─── x   G   J     |
|nnn.....nnn.nnn.ttt.........nnn.nnn...|
|                                      |
|................ttt...................|
|    C           C   C   C             |
|....nnn.........nnn.nnn.nnn...........|
|B   D       F               H   K     |
|nnn.nnn.....nnn.............nnn.nnn...|
|        F                             |
|........nnn...........................|
"#,
};
