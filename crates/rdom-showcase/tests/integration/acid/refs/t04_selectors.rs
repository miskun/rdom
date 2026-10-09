//! Tile 4 — selectors.
//!
//! Spec: Selectors 4 §16 (combinators: descendant, `>`, `+`, `~` — the
//! sibling combinators count elements only, so text between two elements
//! leaves them adjacent), §6 (attribute selectors; §6.3 the `i` / `s`
//! flags), HTML §4.16.2 (`type` among the attributes whose values match
//! ASCII case-insensitively unless the selector says `s`), §14 / §15
//! (structural pseudo-classes; `:empty` — comments are not content;
//! `:nth-child(An+B of S)`), §4 (`:is()`, `:where()`, `:not()` with a
//! complex argument), §4.5 (`:has()` with relative selectors), §7.2
//! (`:link`, `:any-link`; `:visited` never matching here, DIVERGENCES §1
//! "Platform features a terminal app lacks"), §7.1 (`:lang()` by the
//! inherited language, extended ranges `"*-CH"`), §7.2 / HTML §3.2.6.4
//! (`:dir()` reads the element's directionality — `dir=auto` from its first
//! strong character — never CSS `direction`), §3.5 (`:scope` with no
//! scoping root is `:root`); DIVERGENCES §2 "`Dom::root()` is a Fragment"
//! (the root fragment matches `:root`); HTML §15.3.4 (`a[href]` is
//! underlined).
//!
//! Derivation. Each probe has its own rule: green (`#00a000`) when it
//! should match, red when it should not, so a probe is green or keeps the
//! tile's default colour. One line per row (`<div>`), the words separated
//! by single spaces as written:
//!
//! 0. `desc` (`.d1 time` through an `a`) green; `child` (`.d2 > time`)
//!    green; `!child` (`.d3 > time`, a grandchild) default; `deep`
//!    (`.d8 > data a`: the nearest `data` above the `a` is not `.d8`'s
//!    child, the outer one is — the matcher must try both) green; `adj`
//!    (`.d4a + .d4b`) green after its default `.`; `adj-text`
//!    (`.d5a + .d5b`, a text node `t` between) green; `sib`
//!    (`.d6a ~ .d6b`, a `data` between) green; `!adj` (`.d7a + .d7b`, a
//!    `data` between) default.
//! 1. `[data-k=ab]` on `ab`, `~=` on `x ab y`, `|=` on `ab-cd`, `^=` on
//!    `abc`, `$=` on `cab`, `*=` on `xaby`, `[data-k=ab i]` on `AB` and
//!    `[type=a]` on `type=A` (case-insensitive by HTML) all green;
//!    `[type=a s]` on `type=A` and `[data-k=ab]` on `AB` default.
//! 2. `.s1`'s `f` (`:first-child`) and `l` (`:last-child`) green, `m`
//!    default; `.s2`'s `only` (`:only-child`) green; `.s3`'s two children
//!    `x!only` default. `.e1` is `:empty`: its `::before` reads `empty`,
//!    green; `.e2` holds only a comment, still `:empty`: `cmt`, green;
//!    `.e3` holds `x`, not empty: `x`, default, no `bad`. `root`
//!    (`:root .rt`) and `scope` (`:scope .sc`) green.
//! 3. `.n1` `1`–`5` by `:nth-child(2n+1)`: 1, 3, 5 green. `.n2` `a`–`f`
//!    by `:nth-child(even of .x)` — the `.x` are a, c, d, f — c and f
//!    green. `.n3` `1`–`4` by `:nth-last-child(-n+2)`: 3, 4 green. `.n4`
//!    `a b c d e` (`a data a data a`): `a:nth-of-type(2)` is `c`,
//!    `data:nth-last-of-type(1)` is `d`: both green. `.n5` `a b c d`
//!    (`a data a time`): `a:first-of-type` `a`, `a:last-of-type` `c`,
//!    `time:only-of-type` `d` green; `b` default.
//! 4. `is`, `where`, `not` (`:not(.zz)`) and `not-cx` (`:not(.zz .l5)`)
//!    green, `!not` (`.l4:not(.l4)`) default. The links are underlined by
//!    the UA (`u` / `v`): `link` (`:link`) green; `!visited` keeps its blue
//!    — `:visited` never matches; `any` (`:any-link`) green; `!any` has no
//!    `href`, so neither `:any-link` nor the UA underline: default; `list`
//!    (`:visited, :link`) green.
//! 5. Under `lang="de-CH"`: `de` (`:lang(de)`) and `ch` (`:lang("*-CH")`)
//!    green, `!fr` default. `שלום` (`dir=auto`, Hebrew: rtl) matches
//!    `:dir(rtl)`: green, in logical order (DIVERGENCES §1 "No
//!    bidirectional reordering"). `!rtl-css` is `direction: rtl` but its
//!    directionality is its parent's, ltr: default. `rtl` (`dir=rtl`)
//!    green.
//! 6. `ehas` (`:has(.err)` colours the whole anchor, its `e` too) green;
//!    `child` (`:has(> a.sel)`) green, `!child` (the `.sel` a grandchild)
//!    default; `adj` (`:has(+ a.note)`) green before its default `.`;
//!    `sib` (`:has(~ aside .warn)`; the `aside` made inline by the tile)
//!    green before `.w`; `no-img` (`:not(:has(.img))`) green, `!no-img`
//!    default.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "4",
    spec: &[
        "Selectors 4 §4, §6, §7, §14–§16",
        "HTML §4.16.2, §3.2.6.4, §15.3.4",
        "DIVERGENCES §2 Dom::root() is a Fragment",
    ],
    legend: &[
        ('g', "fg #00a000"),
        ('u', "fg #00a000 underline"),
        ('v', "fg #0000c0 underline"),
    ],
    grid: r#"
|desc child !child deep .adj .tadj-text ..sib ..!adj       |
|gggg.ggggg........gggg..ggg...gggggggg...ggg..............|
|eq word dash pre suf sub i-flag type-ci !s-flag !case     |
|gg.gggg.gggg.ggg.ggg.ggg.gggggg.ggggggg...................|
|fml only x!only empty cmt x root scope                    |
|g.g.gggg........ggggg.ggg...gggg.ggggg....................|
|12345 abcdef 1234 abcde abcd                              |
|g.g.g...g..g...gg...gg..g.gg..............................|
|is where not !not not-cx link !visited any !any list      |
|gg.ggggg.ggg......gggggg.uuuu.vvvvvvvv.uuu......uuuu......|
|de ch !fr שלום !rtl-css rtl                               |
|gg.gg.....gggg..........ggg...............................|
|ehas child !child adj. sib.w no-img !no-img               |
|gggg.ggggg........ggg..ggg...gggggg.......................|
"#,
};
