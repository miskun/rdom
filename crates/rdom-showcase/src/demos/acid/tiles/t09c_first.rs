//! Tile 9c — first line, first letter and highlights (CSS Pseudo-Elements
//! 4 §2.2–§2.3, §3; CSS Custom Highlight API 1).
//!
//! A `::first-line` over a block whose first child holds the line; a
//! floated `::first-letter` drop cap taking the opening punctuation; a
//! custom highlight, built from Rust over text split across elements.

use rdom_tui::{Highlight, NodeId, NodeType, Position, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "9c",
    title: "First line, letter & highlights",
    class: "acid-t9c",
    page: 2,
    x: 60,
    y: 1,
    w: 38,
    h: 2,
    markup: r#"
<div class="band"><div class="fl"><p>ab cd ef gh</p></div><p class="dc">“Abc def ghi jkl</p><p class="hs">a ne<b>ed</b>le b</p></div>
"#,
    css: r#"
.acid-t9c .band { display: flex; gap: 1; align-items: flex-start; }
.acid-t9c .fl { width: 10; }
.acid-t9c .fl::first-line { text-transform: uppercase; color: rgb(0, 160, 0); letter-spacing: 1; }
.acid-t9c .dc { width: 12; }
.acid-t9c .dc::first-letter {
  float: left; line-height: 2; padding-right: 1; color: rgb(192, 0, 0); font-weight: bold;
}
.acid-t9c .dc::first-letter:hover { color: rgb(0, 0, 192); }
.acid-t9c .hs { width: 12; }
.acid-t9c ::highlight(search) { background-color: rgb(0, 0, 128); text-decoration: underline; color: rgb(255, 255, 0); }
"#,
    late_css: "",
    setup: Some(highlight_needle),
    script: None,
};

/// Register the `search` highlight over `needle` in the tile's `.hs` text,
/// as a page's find-in-page would: walk the text nodes, find the match in
/// their concatenation and map its ends back to boundary points.
fn highlight_needle(dom: &mut TuiDom, tile: NodeId) {
    let texts: Vec<NodeId> = dom
        .descendants(tile)
        .filter(|&n| dom.node(n).node_type() == NodeType::Text)
        .collect();
    let mut joined = String::new();
    let mut starts = Vec::new();
    for &t in &texts {
        starts.push(joined.len());
        joined.push_str(dom.node(t).data().unwrap_or(""));
    }
    let at = joined.find("needle").expect("the needle is in the tile");
    let point = |offset: usize| {
        let i = starts.iter().rposition(|&s| s <= offset).unwrap();
        Position::new(texts[i], offset - starts[i])
    };
    let range = dom
        .range_between(point(at), point(at + "needle".len()))
        .expect("a range in the tile");
    dom.highlights_mut().set("search", Highlight::new([range]));
}
