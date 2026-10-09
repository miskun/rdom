//! Tile 17 — selection, highlights and `user-select` (CSS UI 4 §6.1, CSS
//! Pseudo-Elements 4 §3, CSS Custom Highlight API 1, DOM §5.3).
//!
//! A selection set by the page's script from one paragraph into the next,
//! across `user-select: none`, `contain` and `all` spans and a generated
//! `::before`, painted by `::selection`; two custom highlights over the
//! second paragraph overlapping each other and the selection — the one
//! registered first with the higher `priority`; a highlight over a word
//! whose text node the load script edits before it (an insertion and a
//! replacement), the live range following the word.

use rdom_tui::{Highlight, NodeId, NodeType, Position, Selection, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "17",
    title: "Selection, highlights & user-select",
    class: "acid-t17",
    page: 4,
    x: 1,
    y: 1,
    w: 38,
    h: 3,
    markup: r#"
<p class="l1">ab<span class="un">NO</span>cd<span class="ct">CT</span>ef<span class="al">ALL</span>gh</p><p class="l2">ijklmnop</p><p class="l3">xyz NEEDLE</p>
"#,
    css: r#"
.acid-t17 .l1::before { content: "<"; }
.acid-t17 .l1::after { content: ">"; }
.acid-t17 .un { user-select: none; }
.acid-t17 .ct { user-select: contain; }
.acid-t17 .ct::before { content: "*"; }
.acid-t17 .al { user-select: all; }
.acid-t17 ::selection { background-color: rgb(0, 0, 128); color: rgb(255, 255, 0); }
.acid-t17 ::highlight(hit) { background-color: rgb(0, 128, 0); color: rgb(255, 255, 255); }
.acid-t17 ::highlight(err) {
  color: rgb(255, 0, 0); text-decoration: underline; text-decoration-color: rgb(0, 255, 255);
}
.acid-t17 ::highlight(mv) { background-color: rgb(128, 0, 128); }
"#,
    late_css: "",
    setup: Some(select_and_highlight),
    script: Some(edit_before_the_needle),
};

/// The text node of the first `sel` in `tile`.
fn text_of(dom: &TuiDom, tile: NodeId, sel: &str) -> NodeId {
    let el = dom
        .query_selector_in(tile, sel)
        .expect("a valid selector")
        .expect("the tile holds it");
    dom.descendants(el)
        .find(|&n| dom.node(n).node_type() == NodeType::Text)
        .expect("a text node")
}

/// The page's script: select from `ab` offset 1 into `ijklmnop` offset 3;
/// register `err` (priority 1) over `lmno`, then `hit` (priority 0) over
/// `jklm`, and `mv` over `NEEDLE`.
fn select_and_highlight(dom: &mut TuiDom, tile: NodeId) {
    let ab = text_of(dom, tile, ".l1");
    let l2 = text_of(dom, tile, ".l2");
    let l3 = text_of(dom, tile, ".l3");
    dom.set_selection(Some(Selection::new(
        Position::new(ab, 1),
        Position::new(l2, 3),
    )));
    let range = |dom: &TuiDom, node: NodeId, from: usize, to: usize| {
        dom.range_between(Position::new(node, from), Position::new(node, to))
            .expect("a range")
    };
    let err = range(dom, l2, 3, 7);
    dom.highlights_mut()
        .set("err", Highlight::new([err]).with_priority(1));
    let hit = range(dom, l2, 1, 5);
    dom.highlights_mut().set("hit", Highlight::new([hit]));
    let needle = range(dom, l3, 4, 10);
    dom.highlights_mut().set("mv", Highlight::new([needle]));
}

/// The load handler: insert `++` at the start of `xyz NEEDLE`, then
/// replace `xyz` with `w` — both before the `mv` range, which moves.
fn edit_before_the_needle(dom: &mut TuiDom, tile: NodeId) {
    let l3 = text_of(dom, tile, ".l3");
    dom.node_mut(l3).edit_text(0, 0, "++").expect("a text node");
    dom.node_mut(l3).edit_text(2, 5, "w").expect("a text node");
}
