//! I1 — pointer over a nested child (tile 34, row 0).
//!
//! Spec: Selectors 4 §9.2 — `:hover` matches the element the pointer
//! designates **and every ancestor of it** (the flat tree); §16.1–§16.4
//! the combinators: `.p:hover + .s`, `.p:hover ~ .t` and `.hv:hover .t`
//! restyle the siblings and the descendant the moment `.p` / `.hv`
//! gain or lose `:hover`. CSS Text Decoration 3 §2 (`underline` in the
//! text colour, the default here).
//!
//! Derivation, from tile 34's rest state (`refs::t34_pointer`):
//!
//! 1. Pointer over `kid` (x 3): the designated element is `.c`; its
//!    ancestors `.p`, `.hv`, the tile and up match `:hover`. `.p:hover`
//!    colours `ab`, `kid` (inherited) and `cd` yellow; `.c:hover` fills
//!    `kid` maroon; `.p:hover + .s` greens `sib`; `.p:hover ~ .t` fills
//!    `far` navy and `.hv:hover .t` underlines it. `.s:hover` does not
//!    match.
//! 2. Pointer over `a` (x 0): `.p` is designated, `.c` no longer
//!    matches — `kid` loses its fill and keeps the yellow; the rest as 1.
//! 3. Pointer over `sib` (x 8): `.s` designated — `.s:hover` red; `.p`
//!    leaves the chain, so `+` / `~` stop matching (no green, no navy);
//!    `.hv` is still an ancestor, so `far` stays underlined.
//! 4. Pointer off the tile (page 60, 30 — nothing there but the page):
//!    the rest state again.

use super::super::super::reference::Reference;
use super::super::super::refs::t34_pointer;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I1",
    title: "Pointer over a nested child",
    page: 10,
    spec: &["Selectors 4 §9.2, §16.1–§16.4"],
    run,
};

const SPEC: &[&str] = &["Selectors 4 §9.2 (ancestors), §16 (combinators)"];

static OVER_KID: Reference = Reference {
    tile: "34",
    spec: SPEC,
    legend: &[
        ('Y', "fg #ffff00"),
        ('M', "fg #ffff00 bg #800000"),
        ('g', "fg #00a000"),
        ('N', "bg #000080 underline"),
    ],
    grid: r#"
|abkidcd sib far                       |
|YYMMMYY.ggg.NNN.......................|
|press log:                            |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
};

static OVER_P: Reference = Reference {
    tile: "34",
    spec: SPEC,
    legend: &[
        ('Y', "fg #ffff00"),
        ('g', "fg #00a000"),
        ('N', "bg #000080 underline"),
    ],
    grid: r#"
|abkidcd sib far                       |
|YYYYYYY.ggg.NNN.......................|
|press log:                            |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
};

static OVER_SIB: Reference = Reference {
    tile: "34",
    spec: SPEC,
    legend: &[('r', "fg #ff0000"), ('U', "underline")],
    grid: r#"
|abkidcd sib far                       |
|........rrr.UUU.......................|
|press log:                            |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
};

fn run(s: &mut Session) {
    s.hover("34", 3, 0);
    s.expect("pointer over kid", &[&OVER_KID]);
    s.hover("34", 0, 0);
    s.expect("pointer over the box's own text", &[&OVER_P]);
    s.hover("34", 8, 0);
    s.expect("pointer over the sibling", &[&OVER_SIB]);
    s.hover_page(60, 30);
    s.expect("pointer off the tile", &[&t34_pointer::REF]);
}
