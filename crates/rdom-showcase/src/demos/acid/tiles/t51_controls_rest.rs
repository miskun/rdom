//! Tile 51 — a range slider, a pixel `outline-offset` and an unchecked
//! default box (HTML §4.10.5.1.13, §4.16.3; CSS UI 4 §5; ACID-STATIC-REST,
//! tile 15a's left-out cases).
//!
//! A range slider at the middle of its range; a word ringed by an
//! `outline` with `outline-offset: 1px`; a checkbox checked by its
//! `checked` attribute that the page's load script unchecks — still
//! `:default`, no longer `:checked`.

use rdom_tui::{NodeId, TuiAccessorsMut, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "51",
    title: "Controls: slider, px offset, default",
    class: "acid-t51",
    page: 13,
    x: 41,
    y: 1,
    w: 38,
    h: 5,
    markup: r#"
<div class="row"><input type="range" class="rg" min="0" max="10" value="5"><b class="po">ab</b><input type="checkbox" class="dc" checked></div>
"#,
    css: r#"
.acid-t51 .row { display: flex; gap: 4; align-items: flex-start; padding: 2 0; }
.acid-t51 .rg { width: 11; }
.acid-t51 .po { font-weight: normal; outline: solid; outline-offset: 1px; }
.acid-t51 .dc:default { color: rgb(255, 255, 0); }
.acid-t51 .dc:checked { color: rgb(0, 160, 0); }
"#,
    late_css: "",
    setup: None,
    script: Some(uncheck),
};

/// The load handler: `checkbox.checked = false`.
fn uncheck(dom: &mut TuiDom, tile: NodeId) {
    let dc = dom
        .query_selector_in(tile, ".dc")
        .expect("a valid selector")
        .expect("the tile holds it");
    dom.node_mut(dc).set_checked(false).unwrap();
}
