//! Tile 48 — a sliding panel (CSS Transforms 2 `translate`, CSS
//! Transitions 1, CSS Animations 1; ACID-INTERACTIVE I20).
//!
//! A button whose `click` listener adds `.open` to the tile: a panel at
//! `translate: -100% 0` transitions to `translate: 0` over 200 ms; under
//! it a box slides in by a `@keyframes` `transform: translateX()` over
//! the same 200 ms, beside a `rotate()` spinner that moves nothing.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "48",
    title: "Slide a panel in",
    class: "acid-t48",
    page: 6,
    x: 61,
    y: 24,
    w: 58,
    h: 3,
    markup: r#"
<button class="go">go</button>
<div class="rw"><div class="sl">0123456789</div></div>
<div class="rw"><div class="kf">abcdefghij</div><div class="sp">S</div></div>
"#,
    css: r#"
@keyframes acid-slide { from { transform: translateX(-100%); } to { transform: translateX(0); } }
@keyframes acid-spin { to { rotate: 360deg; } }
.acid-t48 .rw { display: flex; gap: 2; }
.acid-t48 .sl {
  width: 10; translate: -100% 0; transition: translate 200ms linear;
  background-color: rgb(0, 0, 128);
}
.acid-t48.open .sl { translate: 0; }
.acid-t48 .kf { width: 10; transform: translateX(-100%); background-color: rgb(0, 128, 128); }
.acid-t48.open .kf { transform: none; animation: acid-slide 200ms linear; }
.acid-t48 .sp { animation: acid-spin 200ms linear infinite; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The button's listener: `.open` on the tile.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let go = dom
        .query_selector_in(tile, ".go")
        .expect("a valid selector")
        .expect("the tile holds it");
    dom.add_event_listener(go, "click", ListenerOptions::default(), move |ctx| {
        ctx.dom.add_class(tile, "open").unwrap();
    })
    .unwrap();
}
