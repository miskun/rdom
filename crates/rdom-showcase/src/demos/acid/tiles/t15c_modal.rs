//! Tile 15c — a modal dialog and its `::backdrop` (HTML §4.11.4 `dialog`,
//! §6.3 inertness; CSS Position 4 §3 the top layer; CSS Color 4 §4.2).
//!
//! A modal `<dialog>` shown from inside a 1 × 1 `overflow: hidden` box,
//! centred in the viewport over page text and over a `z-index: 5` box,
//! above every stacking context and outside the box's clip; its
//! translucent `::backdrop` over the whole viewport — the page under it
//! tinted cell by cell. The dialog fills its box with `Canvas`. The
//! whole page is the tile: the backdrop covers every cell.

use rdom_tui::runtime::builtins::dialog;
use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "15c",
    title: "Top layer: modal dialog",
    class: "acid-t15c",
    page: 5,
    x: 0,
    y: 1,
    w: 120,
    h: 49,
    markup: r#"
<div class="dh"><dialog class="dlg"><p>Modal</p></dialog></div><div class="txt">abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij
abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij</div><div class="z"></div>
"#,
    css: r#"
.acid-t15c .dh { width: 1; height: 1; overflow: hidden; }
.acid-t15c .txt { position: absolute; left: 0; top: 18; white-space: pre; }
.acid-t15c .z {
  position: absolute; z-index: 5; left: 50; top: 21; width: 20; height: 3;
  background-color: rgb(0, 100, 0);
}
.acid-t15c .dlg:modal { color: rgb(0, 160, 0); }
.acid-t15c .dlg::backdrop { background-color: rgb(0 0 100 / 60%); }
"#,
    late_css: "",
    setup: None,
    script: Some(show),
};

/// The page's load handler: `dialog.showModal()`.
fn show(dom: &mut TuiDom, tile: NodeId) {
    let dlg = dom
        .query_selector_in(tile, ".dlg")
        .expect("a valid selector")
        .expect("the tile holds the dialog");
    dialog::show_modal(dom, dlg).expect("a dialog");
}
