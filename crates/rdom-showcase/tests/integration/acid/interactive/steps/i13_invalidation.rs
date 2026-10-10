//! I13 — selector invalidation (tile 4; C11-NTH, C11-HAS, C11-LINK-LANG).
//!
//! Spec: Selectors 4 §15 (`:nth-child(An+B)` counts an element's
//! position among its siblings; `of S` among the siblings matching `S`),
//! §4.5 (`:has()` matches while its relative selector matches anything —
//! a descendant at any depth, a following sibling, a hovered or checked
//! descendant), §9.2 (`:hover`), §14.3 (`:checked`), §7.2 / HTML
//! §3.2.6.4 (`:dir()` reads directionality, which `dir=auto` takes from
//! the element's first strong character — recomputed when its text
//! changes); HTML §8.1.7.3 (the next rendering update restyles). A style
//! a selector reads must follow every DOM change that can flip it.
//!
//! Derivation, from tile 4's rest state (`refs::t04_selectors`), each
//! change a script (or the pointer) makes, then the frame after it:
//!
//! 1. `<a>0</a>` inserted before `.n1`'s `1`: the odd positions are now
//!    `0`, `2`, `4` — those green, `1`, `3`, `5` default; the rest of row
//!    3 moves a cell right.
//! 2. `.n2`'s `a` loses `.x`: the `.x` siblings are `c`, `d`, `f`, so
//!    `even of .x` is `d` alone — `c` and `f` lose their green.
//! 3. `.img` added to the `time` two levels inside `.x6`: `:not(:has(.img))`
//!    fails — `no-img` default.
//! 4. `.x4`'s next sibling loses `.note`: `adj` default; it gains `.note`
//!    again: `adj` green.
//! 5. The pointer over `hover`: its `a` is hovered, so `.y1:has(:hover)`
//!    matches — green.
//! 6. A click on the form's checkbox: checked, `.y2:has(:checked)`
//!    matches — `[x] ` and `chk` green; the pointer has left `hover`
//!    (default again).
//! 7. The `dir=auto` span's text `שלום` replaced by `abcd`: its first
//!    strong character is now left-to-right — `:dir(rtl)` fails, `abcd`
//!    default.

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I13",
    title: "Selector invalidation",
    page: 1,
    spec: &["Selectors 4 §4.5, §7.2, §9.2, §14.3, §15; HTML §3.2.6.4"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "Selectors 4 §4.5, §7.2, §9.2, §14.3, §15",
    "HTML §3.2.6.4, §8.1.7.3",
];

const LEGEND: &[(char, &str)] = &[
    ('g', "fg #00a000"),
    ('u', "fg #00a000 underline"),
    ('v', "fg #0000c0 underline"),
];

static INSERTED: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g....g..g...gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg...gggggg.......................|
|hover [ ] chk                                             |
|..........................................................|
"#,
};

static OF_S: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg...gggggg.......................|
|hover [ ] chk                                             |
|..........................................................|
"#,
};

static DEEP: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg................................|
|hover [ ] chk                                             |
|..........................................................|
"#,
};

static NOTE_LOST: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg.............ggg................................|
|hover [ ] chk                                             |
|..........................................................|
"#,
};

static NOTE_BACK: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg................................|
|hover [ ] chk                                             |
|..........................................................|
"#,
};

static HOVER: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg................................|
|hover [ ] chk                                             |
|ggggg.....................................................|
"#,
};

static CHECKED: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg................................|
|hover [x] chk                                             |
|......ggggggg.............................................|
"#,
};

static LATIN: Reference = Reference {
    tile: "4",
    spec: SPEC,
    legend: LEGEND,
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|012345 abcdef 1234 abcde abcd                             |
|g.g.g.....g.....gg...gg..g.gg.............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr abcd !rtl-css rtl                               |
|gg.gg...................ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg................................|
|hover [x] chk                                             |
|......ggggggg.............................................|
"#,
};

fn run(s: &mut Session) {
    let n1 = s.find("4", ".n1");
    s.script(|dom| {
        let first = dom.node(n1).first_child().expect("an item").id();
        let a = dom.create_element("a");
        let zero = dom.create_text_node("0");
        dom.append_child(a, zero).unwrap();
        dom.insert_before(n1, a, Some(first)).unwrap();
    });
    s.expect(
        "a sibling inserted before the nth-child items",
        &[&INSERTED],
    );
    let n2 = s.find("4", ".n2");
    s.script(|dom| {
        let first = dom.node(n2).first_child().expect("an item").id();
        dom.remove_class(first, "x").unwrap();
    });
    s.expect("an of-S sibling loses its class", &[&OF_S]);
    let dp = s.find("4", ".dp");
    s.script(|dom| dom.add_class(dp, "img").unwrap());
    s.expect("a class added deep inside a :has() anchor", &[&DEEP]);
    let note = s.find("4", ".note");
    s.script(|dom| {
        dom.remove_class(note, "note").unwrap();
    });
    s.expect("the next sibling loses .note", &[&NOTE_LOST]);
    s.script(|dom| dom.add_class(note, "note").unwrap());
    s.expect("the next sibling gains .note", &[&NOTE_BACK]);
    s.hover("4", 1, 7);
    s.expect("the pointer enters the :has(:hover) row", &[&HOVER]);
    s.click("4", 7, 7);
    s.expect(
        "the checkbox in the :has(:checked) form checked",
        &[&CHECKED],
    );
    let h1 = s.find("4", ".h1");
    s.script(|dom| dom.set_text_content(h1, "abcd").unwrap());
    s.expect("the dir=auto text edited to Latin", &[&LATIN]);
}
