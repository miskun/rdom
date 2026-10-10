//! Tile 41 — scroll snapping, overscroll and keyboard scrolling (CSS
//! Scroll Snap 1, CSS Overscroll Behavior 1, CSSOM View §5.1;
//! ACID-INTERACTIVE I11).
//!
//! Seven 4-wide scroll containers in a row, none drawing a scrollbar
//! (`scrollbar-width: none`): a `y mandatory` list of one-row items whose
//! fifth is `scroll-snap-stop: always`; a `y proximity` list of two
//! six-row items; an outer box holding an `overscroll-behavior: contain`
//! scroller and a plain, focusable one; a list of focusable rows under
//! `scroll-padding: 1 0`; a `y mandatory` list a script inserts a row
//! into; a `y mandatory` list whose middle item (8 rows) is taller than
//! its 3-row snapport; a `y mandatory` list of 4-row items under
//! `scroll-behavior: smooth`.

use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "41",
    title: "Scroll snapping and chaining",
    class: "acid-t41",
    page: 11,
    x: 1,
    y: 1,
    w: 38,
    h: 4,
    markup: r#"
<div class="row"><div class="box m" tabindex="0"><div>m0</div><div>m1</div><div>m2</div><div>m3</div><div class="st">m4</div><div>m5</div><div>m6</div><div>m7</div><div>m8</div><div>m9</div></div><div class="box p"><div><div>a0</div><div>a1</div><div>a2</div><div>a3</div><div>a4</div><div>a5</div></div><div><div>b0</div><div>b1</div><div>b2</div><div>b3</div><div>b4</div><div>b5</div></div></div><div class="box oo"><div class="ic"><div>c0</div><div>c1</div><div>c2</div></div><div class="ic2" tabindex="0"><div>d0</div><div>d1</div><div>d2</div></div><div>o3</div><div>o4</div><div>o5</div><div>o6</div><div>o7</div><div>o8</div></div><div class="box tp"><div tabindex="0">t0</div><div tabindex="0">t1</div><div tabindex="0">t2</div><div tabindex="0">t3</div><div tabindex="0">t4</div><div tabindex="0">t5</div></div><div class="box rs"><div>r0</div><div>r1</div><div>r2</div><div>r3</div><div>r4</div><div>r5</div></div><div class="box tc" tabindex="0"><div><div>A0</div><div>A1</div><div>A2</div></div><div><div>T0</div><div>T1</div><div>T2</div><div>T3</div><div>T4</div><div>T5</div><div>T6</div><div>T7</div></div><div><div>B0</div><div>B1</div><div>B2</div></div></div><div class="box sm" tabindex="0"><div><div>x0</div><div>x1</div><div>x2</div><div>x3</div></div><div><div>y0</div><div>y1</div><div>y2</div><div>y3</div></div><div><div>z0</div><div>z1</div><div>z2</div><div>z3</div></div></div></div>
"#,
    css: r#"
.acid-t41 .row { display: flex; gap: 1; align-items: flex-start; }
.acid-t41 .box { width: 4; height: 3; overflow: auto; scrollbar-width: none; }
.acid-t41 .ic, .acid-t41 .ic2 { height: 2; overflow: auto; scrollbar-width: none; }
.acid-t41 .oo, .acid-t41 .sm { height: 4; }
.acid-t41 .m, .acid-t41 .rs, .acid-t41 .tc, .acid-t41 .sm { scroll-snap-type: y mandatory; }
.acid-t41 .p { scroll-snap-type: y proximity; }
.acid-t41 .m > div, .acid-t41 .p > div, .acid-t41 .rs > div, .acid-t41 .tc > div, .acid-t41 .sm > div {
  scroll-snap-align: start;
}
.acid-t41 .m .st { scroll-snap-stop: always; }
.acid-t41 .ic { overscroll-behavior: contain; }
.acid-t41 .tp { scroll-padding: 1 0; }
.acid-t41 .sm { scroll-behavior: smooth; }
"#,
    late_css: "",
    setup: None,
    script: None,
};

/// Step I11's insertion: a row before the first of the re-snapping list.
pub fn insert_row(dom: &mut TuiDom, tile: NodeId) {
    let list = dom
        .query_selector_in(tile, ".rs")
        .expect("a valid selector")
        .expect("the tile holds it");
    let first = dom.node(list).first_child().expect("a row").id();
    let row = dom.create_element("div");
    let text = dom.create_text_node("new");
    dom.append_child(row, text).unwrap();
    dom.insert_before(list, row, Some(first)).unwrap();
}
