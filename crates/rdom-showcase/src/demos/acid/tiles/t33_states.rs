//! Tile 33 — states and the caret (Selectors 4 §9–§14, HTML §4.16.3, CSS
//! UI 4 §6.2, CSS Pseudo-Elements 4 §4; ACID-COVERAGE).
//!
//! A text field the load script focuses — `:focus`, `:focus-visible` (a
//! text field's focus is always evident), its wrapper's `:focus-within` —
//! with its caret styled by `caret` and its longhands; a button the script
//! makes active (`:active`, as the runtime does on a press); `:checked`,
//! `:placeholder-shown`, `:open`, `:enabled`, `:valid`, `:required` and
//! `:optional`; a list-item `::after` whose `::after::marker` is styled.

use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "33",
    title: "States and the caret",
    class: "acid-t33",
    page: 9,
    x: 61,
    y: 1,
    w: 58,
    h: 2,
    markup: r#"
<div class="band"><div class="fw"><input class="fi" value="ab"></div><button class="ab">A</button><input type="checkbox" class="ck" checked><input class="phs" placeholder="ph"><details class="dop" open><summary>s</summary>x</details><button class="en">E</button><input class="va" value="v"><input class="rq2" required value="r"><input class="op"><div class="am"></div></div>
"#,
    css: r#"
.acid-t33 .band { display: flex; gap: 1; align-items: flex-start; }
.acid-t33 .fw { padding: 0 1; }
.acid-t33 .fw:focus-within { background-color: rgb(128, 0, 0); }
.acid-t33 .fi {
  width: 4;
  caret: auto; caret-color: rgb(255, 0, 0); caret-shape: bar; caret-animation: manual;
  caret-text-color: rgb(255, 255, 255);
}
.acid-t33 .fi:focus { text-decoration: underline; }
.acid-t33 .fi:focus-visible { color: rgb(255, 255, 0); }
.acid-t33 .ab:active { color: rgb(255, 0, 0); }
.acid-t33 .ck:checked { color: rgb(0, 160, 0); }
.acid-t33 .phs { width: 4; }
.acid-t33 .phs:placeholder-shown { background-color: rgb(128, 128, 0); }
.acid-t33 .dop { width: 3; }
.acid-t33 .dop:open { color: rgb(0, 160, 0); }
.acid-t33 .en:enabled { color: rgb(0, 160, 0); }
.acid-t33 .va, .acid-t33 .rq2, .acid-t33 .op { width: 2; }
.acid-t33 .va:valid { background-color: rgb(0, 128, 128); }
.acid-t33 .rq2:required { background-color: rgb(0, 0, 128); }
.acid-t33 .op:optional { background-color: rgb(128, 0, 0); }
.acid-t33 .am::after { content: "z"; display: list-item; list-style-position: inside; }
.acid-t33 .am::after::marker { color: rgb(255, 0, 0); }
"#,
    late_css: "",
    setup: None,
    script: Some(focus_and_press),
};

/// The load handler: `input.focus()`, and the button pressed — the state
/// the runtime gives a pressed element (`Dom::set_active`).
fn focus_and_press(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let fi = find(dom, ".fi");
    rdom_tui::runtime::focus::focus_node(dom, Some(fi));
    let ab = find(dom, ".ab");
    dom.set_active(Some(ab));
}
