//! I12 — pointer over and pressing generated boxes (tiles 9a–9c;
//! C10-PSEUDO-CHAINS, C10G-MARKER-HIT).
//!
//! Spec: Selectors 4 §3.6.3 — a pseudo-element followed by a user-action
//! pseudo-class is represented only while it is in that state: the
//! pointer designates the pseudo-element's own box (`::before:hover`
//! while the pointer is over the `::before`'s cells, not over its host's
//! text — the host is hovered there, its pseudo-elements are not its
//! descendants in the flat tree), `::after:active` while the primary
//! button is held on it; a `::before` is never focused, so
//! `::before:focus` never matches, its host focused or not. CSS Lists 3
//! §3.1 (`::marker`, inside and outside — an outside marker is hit where
//! it is drawn, C10G-MARKER-HIT), CSS Pseudo-Elements 4 §2.3
//! (`::first-letter`, here a floated drop cap). HTML §6.6 (a press on a
//! `tabindex` element focuses it).
//!
//! Derivation, from page 2's rest state (each tile's static reference):
//!
//! 1. Pointer over `<` (9a, x 0): the `::before` `<ATTR:` red; `mid` and
//!    `>` plain.
//! 2. Over `m` (x 6): the host is hovered, its `::before` is not — the
//!    rest state.
//! 3. Button down on `>` (x 9): the `::after` is active — green; the
//!    host is focused (`tabindex`), and `::before:focus` stays unmatched:
//!    `<ATTR:` plain.
//! 4. Released: the rest state.
//! 5. Over the outside marker `•` of `red` (9b, x 2, row 6): the marker
//!    blue (`::marker:hover`), `red` plain.
//! 6. Over `red`: the marker back to its red.
//! 7. Over the inside marker `3.` (x 19, row 3): `3.` blue.
//! 8. Over `dd` (x 22): the rest state.
//! 9. Over the drop cap's `A` (9c, x 12): `“A` blue, still bold.
//! 10. Over `bc` (x 14): the rest state.

use super::super::super::reference::Reference;
use super::super::super::refs::{t09a_generated, t09b_lists, t09c_first};
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I12",
    title: "Pointer over and pressing generated boxes",
    page: 2,
    spec: &["Selectors 4 §3.6.3; CSS Lists 3 §3.1; CSS Pseudo-Elements 4 §2.3"],
    run,
    configure: None,
};

const SPEC_9A: &[&str] = &["Selectors 4 §3.6.3; CSS Generated Content 3 §1; HTML §6.6"];
const SPEC_9B: &[&str] = &["Selectors 4 §3.6.3; CSS Lists 3 §3.1; C10G-MARKER-HIT"];
const SPEC_9C: &[&str] = &["Selectors 4 §3.6.3; CSS Pseudo-Elements 4 §2.3"];

static A_BEFORE: Reference = Reference {
    tile: "9a",
    spec: SPEC_9A,
    legend: &[
        ('r', "fg #c00000"),
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('p', "bg #800080"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|<ATTR:mid> ★alt S:one                                     |
|rrrrrr....................................................|
|                                                          |
|..........................................................|
|1 A       1 x   XII μ יב 一二 • 007 (3) 99 a              |
|..........................................................|
|1.1 B     2 y                                             |
|..........................................................|
|1.2 C     7 w                                             |
|..........................................................|
|1.2.1 D   8 v                                             |
|..........................................................|
|          1 z                                             |
|..........................................................|
|                                                          |
|..........................................................|
|«a‹b›» „x“ «y» 「z」 w BLOCK  aa [bb                      |
|..........................................................|
|                       text   cc] dd                      |
|..........................................................|
|                                                          |
|..........................................................|
|HOST     AAaaSS      xRz                                  |
|nnnn.......ttpp...........................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|▸ closed             ▾ open                               |
|..........................................................|
|                     shown                                |
|.....................ggggg................................|
|                     para                                 |
|.....................gggg.................................|
"#,
};

static A_PRESSED: Reference = Reference {
    tile: "9a",
    spec: SPEC_9A,
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('p', "bg #800080"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|<ATTR:mid> ★alt S:one                                     |
|.........g................................................|
|                                                          |
|..........................................................|
|1 A       1 x   XII μ יב 一二 • 007 (3) 99 a              |
|..........................................................|
|1.1 B     2 y                                             |
|..........................................................|
|1.2 C     7 w                                             |
|..........................................................|
|1.2.1 D   8 v                                             |
|..........................................................|
|          1 z                                             |
|..........................................................|
|                                                          |
|..........................................................|
|«a‹b›» „x“ «y» 「z」 w BLOCK  aa [bb                      |
|..........................................................|
|                       text   cc] dd                      |
|..........................................................|
|                                                          |
|..........................................................|
|HOST     AAaaSS      xRz                                  |
|nnnn.......ttpp...........................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|▸ closed             ▾ open                               |
|..........................................................|
|                     shown                                |
|.....................ggggg................................|
|                     para                                 |
|.....................gggg.................................|
"#,
};

static B_OUTSIDE: Reference = Reference {
    tile: "9b",
    spec: SPEC_9B,
    legend: &[
        ('r', "fg #c00000"),
        ('g', "fg #00a000"),
        ('b', "fg #0000c0"),
    ],
    grid: r#"
| 9. a           1. aa bb cc   •   ◦   ▪ x   2. a          |
|..........................................................|
|10. b              dd               ab •    1. b          |
|..........................................................|
|II. c           2.                          7. b          |
|..........................................................|
| III. c            3. dd ee                 8. c          |
|..........................................................|
|                                            a. x          |
|..........................................................|
|                                                          |
|..........................................................|
|  • red      ▪ B      x • yy                              |
|..b..........g............................................|
|  → txt        body                                       |
|..r.......................................................|
"#,
};

static B_INSIDE: Reference = Reference {
    tile: "9b",
    spec: SPEC_9B,
    legend: &[
        ('r', "fg #c00000"),
        ('g', "fg #00a000"),
        ('b', "fg #0000c0"),
    ],
    grid: r#"
| 9. a           1. aa bb cc   •   ◦   ▪ x   2. a          |
|..........................................................|
|10. b              dd               ab •    1. b          |
|..........................................................|
|II. c           2.                          7. b          |
|..........................................................|
| III. c            3. dd ee                 8. c          |
|...................bb.....................................|
|                                            a. x          |
|..........................................................|
|                                                          |
|..........................................................|
|  • red      ▪ B      x • yy                              |
|..r..........g............................................|
|  → txt        body                                       |
|..r.......................................................|
"#,
};

static C_LETTER: Reference = Reference {
    tile: "9c",
    spec: SPEC_9C,
    legend: &[
        ('g', "fg #00a000"),
        ('B', "fg #0000c0 bold"),
        ('h', "fg #ffff00 bg #000080 underline"),
        ('H', "fg #ffff00 bg #000080 underline bold"),
    ],
    grid: r#"
|A B   C D  “A bc def    a needle b    |
|g.g...g.g..BB.............hhHHhh......|
|ef gh         ghi jkl                 |
|......................................|
"#,
};

fn run(s: &mut Session) {
    s.hover("9a", 0, 0);
    s.expect("over the ::before", &[&A_BEFORE]);
    s.hover("9a", 6, 0);
    s.expect("over the host's text", &[&t09a_generated::REF]);
    s.hover("9a", 9, 0);
    s.press("9a", 9, 0);
    s.expect("pressing the ::after", &[&A_PRESSED]);
    let g1 = s.find("9a", ".g1");
    let focused = s.dom().focused();
    s.check_eq(
        "the host focused by the press",
        focused,
        Some(g1),
        "HTML §6.6",
    );
    s.release("9a", 9, 0);
    s.expect("released", &[&t09a_generated::REF]);
    s.hover("9b", 2, 6);
    s.expect("over the outside marker", &[&B_OUTSIDE]);
    s.hover("9b", 4, 6);
    s.expect("over its item's text", &[&t09b_lists::REF]);
    s.hover("9b", 19, 3);
    s.expect("over the inside marker", &[&B_INSIDE]);
    s.hover("9b", 22, 3);
    s.expect("over its item's text", &[&t09b_lists::REF]);
    s.hover("9c", 12, 0);
    s.expect("over the drop cap", &[&C_LETTER]);
    s.hover("9c", 14, 0);
    s.expect("over the text beside it", &[&t09c_first::REF]);
}
