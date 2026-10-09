//! Tile 21 — feature queries (CSS Conditional 3 §6, 4 §6, 5 §5; CSS
//! Cascade 5 §3 `@import … supports()`).
//!
//! One word per condition, coloured green when rdom's parser answers it
//! true: a supported and an unsupported declaration, a pixel geometry
//! value, `not` / `and` / `or`, `selector()` with a supported and an
//! unparsed selector, `font-tech()` / `font-format()`, a
//! `<general-enclosed>` and its `not`; `@supports` nested in a style rule
//! and inside `@media`; an `@import` with a `supports()` condition true
//! and one false (the page's loader, `IMPORT`).

use super::super::Tile;

/// The sheets this tile's `<style>` elements `@import`, by URL.
pub const IMPORT: &[(&str, &str)] = &[
    ("acid-t21.css", ".acid-t21 .imp { color: rgb(0, 160, 0); }"),
    (
        "acid-t21-no.css",
        ".acid-t21 .impn { color: rgb(0, 160, 0); }",
    ),
];

pub const TILE: Tile = Tile {
    id: "21",
    title: "Feature queries",
    class: "acid-t21",
    page: 6,
    x: 61,
    y: 1,
    w: 58,
    h: 2,
    markup: r#"
<style>@import url("acid-t21.css") supports(display: grid);</style><style>@import url("acid-t21-no.css") supports(display: frob);</style><div><b class="grid">grid</b> <b class="frob">frob</b> <b class="wpx">wpx</b> <b class="notf">notf</b> <b class="and">and</b> <b class="or">or</b> <b class="has">has</b> <b class="sel">sel</b> <b class="tech">tech</b> <b class="fmt">fmt</b> <b class="ge">ge</b> <b class="nge">nge</b> <b class="nest">nest</b> <b class="inm">inm</b> <b class="imp">imp</b> <b class="impn">impn</b></div>
"#,
    css: r#"
.acid-t21 b { font-weight: normal; }
@supports (display: grid) { .acid-t21 .grid { color: rgb(0, 160, 0); } }
@supports (display: frob) { .acid-t21 .frob { color: rgb(0, 160, 0); } }
@supports (width: 10px) { .acid-t21 .wpx { color: rgb(0, 160, 0); } }
@supports not (display: frob) { .acid-t21 .notf { color: rgb(0, 160, 0); } }
@supports (display: grid) and (color: red) { .acid-t21 .and { color: rgb(0, 160, 0); } }
@supports (display: frob) or (color: red) { .acid-t21 .or { color: rgb(0, 160, 0); } }
@supports selector(:has(+ p)) { .acid-t21 .has { color: rgb(0, 160, 0); } }
@supports selector(:frob) { .acid-t21 .sel { color: rgb(0, 160, 0); } }
@supports font-tech(color-COLRv1) { .acid-t21 .tech { color: rgb(0, 160, 0); } }
@supports font-format(woff2) { .acid-t21 .fmt { color: rgb(0, 160, 0); } }
@supports frob(1) { .acid-t21 .ge { color: rgb(0, 160, 0); } }
@supports not frob(1) { .acid-t21 .nge { color: rgb(0, 160, 0); } }
.acid-t21 .nest { @supports (display: grid) { color: rgb(0, 160, 0); } }
@media (min-width: 1) { @supports (display: flex) { .acid-t21 .inm { color: rgb(0, 160, 0); } } }
"#,
    late_css: "",
    setup: None,
    script: None,
};
