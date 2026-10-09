//! Tile 15b — the top layer, without a modal (HTML §6.4 popovers, §4.10.7
//! the select picker; CSS Position 4 §3 the top layer).
//!
//! An open drop-down `<select>` inside an `overflow: hidden` box: its
//! picker overlays the page text below it from the top layer, outside the
//! box's clip, without moving anything. A shown `popover` centred in the
//! viewport over the same text, its `Canvas` background blanking every
//! cell of its box — padding included — and a `[popover]` sibling hidden.
//! The page's load script opens the select and shows the popover.

use rdom_tui::runtime::builtins::{popover, select};
use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "15b",
    title: "Top layer: picker & popover",
    class: "acid-t15b",
    page: 4,
    x: 30,
    y: 14,
    w: 60,
    h: 22,
    markup: r#"
<div class="txt">012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789
012345678901234567890123456789012345678901234567890123456789</div><div class="sh"><select class="so"><option>One</option><option selected>Two</option><option>Three</option></select></div><div popover class="pop">Popover</div><div popover>hidden</div>
"#,
    css: r#"
.acid-t15b .txt { white-space: pre; }
.acid-t15b .sh { position: absolute; left: 2; top: 1; width: 12; height: 1; overflow: hidden; }
.acid-t15b .so { width: 12; }
.acid-t15b .pop { padding: 0 1; }
.acid-t15b .pop:popover-open { color: rgb(0, 160, 0); }
"#,
    late_css: "",
    setup: None,
    script: Some(open_layer),
};

/// The page's load handler: open the select's picker and show the
/// popover (`select.showPicker()`, `popover.showPopover()`).
fn open_layer(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let so = find(dom, ".so");
    select::open(dom, so);
    let pop = find(dom, ".pop");
    popover::show_popover(dom, pop).expect("a popover");
}
