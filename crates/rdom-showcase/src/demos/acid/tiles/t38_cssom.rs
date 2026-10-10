//! Tile 38 — CSSOM rewrites (HTML §4.2.6, CSSOM §6.7; ACID-INTERACTIVE
//! I7).
//!
//! A live `<style>` element colouring two words red, beside the page's
//! own sheet (the `App`'s) colouring the second blue — the App's sheets
//! cascade after the document's `<style>` sheets (DIVERGENCES §2) — and
//! a word whose `style` attribute colours it red. Step I7 rewrites the
//! `<style>` element's text and the inline style through
//! `CSSStyleDeclaration`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "38",
    title: "CSSOM rewrite",
    class: "acid-t38",
    page: 10,
    x: 61,
    y: 6,
    w: 38,
    h: 1,
    markup: r#"
<style class="st">.acid-t38 .a { color: rgb(255, 0, 0); } .acid-t38 .b { color: rgb(255, 0, 0); }</style><span class="a">sty</span> <span class="b">app</span> <span class="c" style="color: rgb(255, 0, 0)">inl</span>
"#,
    css: r#"
.acid-t38 .b { color: rgb(0, 0, 255); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
