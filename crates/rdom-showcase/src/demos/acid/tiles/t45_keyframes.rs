//! Tile 45 — keyframe animations on the clock (CSS Animations 1 §3–§4,
//! 2 §4.2; Web Animations 1 §4–§5; ACID-INTERACTIVE I17).
//!
//! Nothing runs at rest. Step I17's script adds `.run` to the tile, which
//! names an animation on each box: row 0 swatches — three keyframes with
//! a per-keyframe `steps()` easing, an `alternate` two-iteration run (its
//! events logged on row 4), a `forwards` fill, a delayed `backwards`
//! fill, an animation over a transition of the same property, and one
//! the step cancels by removing its class (logged on row 5); row 1 a
//! keyframe `width`; row 2 the same against an `!important` width; row 3
//! `animation-composition: add` over the box's own width.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "45",
    title: "Keyframes on the clock",
    class: "acid-t45",
    page: 12,
    x: 1,
    y: 13,
    w: 58,
    h: 6,
    markup: r#"
<div class="sw"><div class="k1"></div><div class="k2"></div><div class="k3"></div><div class="k4"></div><div class="kt"></div><div class="kc kca"></div></div>
<div class="kw"></div>
<div class="ki"></div>
<div class="ka"></div>
<div class="lg1">log:</div>
<div class="lg2">log:</div>
"#,
    css: r#"
@keyframes acid-k1 {
  from { background-color: rgb(0, 0, 0); animation-timing-function: steps(2, jump-end); }
  50% { background-color: rgb(100, 0, 0); }
  to { background-color: rgb(200, 0, 0); }
}
@keyframes acid-k2 { from { background-color: rgb(0, 0, 0); } to { background-color: rgb(0, 200, 0); } }
@keyframes acid-k3 { from { background-color: rgb(0, 0, 40); } to { background-color: rgb(0, 0, 200); } }
@keyframes acid-kt { from, to { background-color: rgb(200, 0, 0); } }
@keyframes acid-grow { from { width: 2; } to { width: 10; } }
@keyframes acid-addw { from { width: 0; } to { width: 4; } }
.acid-t45 .sw { display: flex; gap: 1; }
.acid-t45 .sw > div { width: 4; height: 1; background-color: rgb(64, 64, 64); }
.acid-t45 .kt { transition: background-color 200ms linear; }
.acid-t45 .kw, .acid-t45 .ki, .acid-t45 .ka { height: 1; background-color: rgb(0, 0, 128); }
.acid-t45 .kw { width: 1; }
.acid-t45 .ki { width: 3 !important; }
.acid-t45 .ka { width: 4; background-color: rgb(0, 128, 128); }
.acid-t45.run .k1 { animation: acid-k1 100ms linear; }
.acid-t45.run .k2 { animation: acid-k2 100ms linear 2 alternate; }
.acid-t45.run .k3 { animation: acid-k3 100ms linear forwards; }
.acid-t45.run .k4 { animation: acid-k3 100ms linear 50ms backwards; }
.acid-t45.run .kt { background-color: rgb(0, 0, 200); animation: acid-kt 100ms linear; }
.acid-t45.run .kca { animation: acid-k3 100ms linear; }
.acid-t45.run .kw, .acid-t45.run .ki { animation: acid-grow 100ms linear; }
.acid-t45.run .ka { animation: acid-addw 100ms linear forwards; animation-composition: add; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The event logs: the alternate swatch's start / iteration / end (with
/// their `elapsedTime`), the cancelled one's cancel.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let logged = [
        (".k2", ".lg1", "animationstart", "s"),
        (".k2", ".lg1", "animationiteration", "i"),
        (".k2", ".lg1", "animationend", "e"),
        (".kc", ".lg2", "animationcancel", "c"),
    ];
    for (target, log, ty, letter) in logged {
        let (target, log) = (find(dom, target), find(dom, log));
        dom.add_event_listener(target, ty, ListenerOptions::default(), move |ctx| {
            let elapsed = ctx
                .event
                .detail
                .as_animation()
                .expect("an animation event")
                .elapsed;
            let mut text = ctx.dom.text_content(log) + letter;
            if letter == "i" || letter == "e" {
                text.push_str(&format!("{elapsed:.1}"));
            }
            ctx.dom.set_text_content(log, &text).unwrap();
        })
        .unwrap();
    }
}
