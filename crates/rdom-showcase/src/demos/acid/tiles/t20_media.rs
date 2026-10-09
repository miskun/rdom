//! Tile 20 — media queries (Media Queries 4 / 5, HTML §4.2.6 `<style
//! media>`, CSS Cascade 5 §3 `@import`).
//!
//! One word per query, coloured green when it matches the page's 120 × 50
//! viewport: ranges, `min-` / `max-`, `and` / `or` / a comma list / `not`,
//! `only`, `print`, an unknown feature alone and negated, a `600px`
//! breakpoint and its `not`, a narrow console `(grid) and (max-width:
//! 15em)`, an `ex` value, `prefers-color-scheme` beside `light-dark()`
//! and the `:root` dark-mode variable pattern (in a `<style>` element:
//! the showcase keeps its sheets' rules class-scoped), `hover` / `pointer` /
//! `grid` / `color`; `@media` inside `@layer` and nested in a style rule;
//! a `<style media>` sheet; an `@import` with a media list (resolved by
//! the page's loader, `IMPORT`).

use super::super::Tile;

/// The sheets this tile's `<style>` elements `@import`, by URL.
pub const IMPORT: &[(&str, &str)] = &[
    ("acid-t20.css", ".acid-t20 .imp { color: rgb(0, 160, 0); }"),
    (
        "acid-t20-print.css",
        ".acid-t20 .impp { color: rgb(0, 160, 0); }",
    ),
];

pub const TILE: Tile = Tile {
    id: "20",
    title: "Media queries",
    class: "acid-t20",
    page: 6,
    x: 1,
    y: 1,
    w: 58,
    h: 3,
    markup: r#"
<style>:root { --acid-t20-c: rgb(160, 0, 0); } @media (prefers-color-scheme: dark) { :root { --acid-t20-c: rgb(0, 160, 0); } }</style><style media="(min-width: 1)">.acid-t20 .smed { color: rgb(0, 160, 0); }</style><style media="print">.acid-t20 .sprn { color: rgb(0, 160, 0); }</style><style>@import url("acid-t20.css") (min-width: 1);</style><style>@import url("acid-t20-print.css") print;</style><div class="w"><b class="range">range</b> <b class="minw">minw</b> <b class="maxw">maxw</b> <b class="and">and</b> <b class="list">list</b> <b class="or">or</b> <b class="notp">notp</b> <b class="print">print</b> <b class="only">only</b> <b class="unk">unk</b> <b class="nunk">nunk</b> <b class="px">px</b> <b class="npx">npx</b> <b class="nar">nar</b> <b class="ex">ex</b> <b class="dark">dark</b> <b class="ld">ld</b> <b class="root">root</b> <b class="hov">hov</b> <b class="ptr">ptr</b> <b class="grd">grd</b> <b class="col">col</b> <b class="lay">lay</b> <b class="nest">nest</b> <b class="smed">smed</b> <b class="sprn">sprn</b> <b class="imp">imp</b> <b class="impp">impp</b></div>
"#,
    css: r#"
.acid-t20 b { font-weight: normal; }
@media (40 <= width < 130) { .acid-t20 .range { color: rgb(0, 160, 0); } }
@media (min-width: 100) { .acid-t20 .minw { color: rgb(0, 160, 0); } }
@media (max-width: 99) { .acid-t20 .maxw { color: rgb(0, 160, 0); } }
@media screen and (orientation: landscape) { .acid-t20 .and { color: rgb(0, 160, 0); } }
@media (width < 10), (height > 40) { .acid-t20 .list { color: rgb(0, 160, 0); } }
@media (width < 10) or (height > 40) { .acid-t20 .or { color: rgb(0, 160, 0); } }
@media not print { .acid-t20 .notp { color: rgb(0, 160, 0); } }
@media print { .acid-t20 .print { color: rgb(0, 160, 0); } }
@media only screen { .acid-t20 .only { color: rgb(0, 160, 0); } }
@media (frob) { .acid-t20 .unk { color: rgb(0, 160, 0); } }
@media not (frob) { .acid-t20 .nunk { color: rgb(0, 160, 0); } }
@media (min-width: 600px) { .acid-t20 .px { color: rgb(0, 160, 0); } }
@media not (min-width: 600px) { .acid-t20 .npx { color: rgb(0, 160, 0); } }
@media (grid) and (max-width: 15em) { .acid-t20 .nar { color: rgb(0, 160, 0); } }
@media (min-width: 2ex) { .acid-t20 .ex { color: rgb(0, 160, 0); } }
@media (prefers-color-scheme: dark) { .acid-t20 .dark { color: rgb(0, 160, 0); } }
@media (hover: hover) { .acid-t20 .hov { color: rgb(0, 160, 0); } }
@media (pointer: fine) { .acid-t20 .ptr { color: rgb(0, 160, 0); } }
@media (grid) { .acid-t20 .grd { color: rgb(0, 160, 0); } }
@media (color >= 8) { .acid-t20 .col { color: rgb(0, 160, 0); } }
.acid-t20 .ld { color: light-dark(rgb(160, 0, 0), rgb(0, 160, 0)); }
.acid-t20 .root { color: var(--acid-t20-c); }
@layer acid-t20 { @media (min-width: 1) { .acid-t20 .lay { color: rgb(0, 160, 0); } } }
.acid-t20 .nest { @media (min-width: 1) { color: rgb(0, 160, 0); } }
"#,
    late_css: "",
    setup: None,
    script: None,
};
