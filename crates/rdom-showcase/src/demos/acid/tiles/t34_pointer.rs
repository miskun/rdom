//! Tile 34 — hover, press and `pointer-events` (Selectors 4 §9.2 /
//! §9.4, HTML §4.16.3, CSS UI 4 §4.4, UI Events §3.4; ACID-INTERACTIVE
//! I1, I2, I10).
//!
//! Row 0, hover chains: a box whose `:hover` colours its text, a child
//! whose own `:hover` fills it, and siblings reading the box's `:hover`
//! through `+` and `~`, and the row's through a descendant combinator.
//! Row 1, `:active` timing: a box filled while it is pressed, and a log
//! its `mouseup` and `click` listeners write — the event's letter, then
//! `+` when the box matched `:active` as the listener ran, else `-`.
//! Row 2, `pointer-events: none`: two boxes under two overlays that draw
//! nothing, the first overlay `pointer-events: none`; a click on each box
//! counts into its counter.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "34",
    title: "Hover, press, pointer-events",
    class: "acid-t34",
    page: 10,
    x: 1,
    y: 1,
    w: 38,
    h: 3,
    markup: r#"
<div class="hv"><div class="p">ab<span class="c">kid</span>cd</div><div class="s">sib</div><div class="t">far</div></div>
<div class="pr"><div class="b">press</div><div class="log">log:</div></div>
<div class="pe"><div class="u1">hit</div><div class="u2">blk</div><div class="n1">0</div><div class="n2">0</div><div class="o1"></div><div class="o2"></div></div>
"#,
    css: r#"
.acid-t34 .hv, .acid-t34 .pr, .acid-t34 .pe { display: flex; gap: 1; }
.acid-t34 .pe { position: relative; }
.acid-t34 .p:hover { color: rgb(255, 255, 0); }
.acid-t34 .c:hover { background-color: rgb(128, 0, 0); }
.acid-t34 .p:hover + .s { color: rgb(0, 160, 0); }
.acid-t34 .p:hover ~ .t { background-color: rgb(0, 0, 128); }
.acid-t34 .hv:hover .t { text-decoration: underline; }
.acid-t34 .s:hover { color: rgb(255, 0, 0); }
.acid-t34 .b:active { background-color: rgb(255, 0, 0); }
.acid-t34 .o1, .acid-t34 .o2 { position: absolute; top: 0; width: 3; height: 1; }
.acid-t34 .o1 { left: 0; pointer-events: none; }
.acid-t34 .o2 { left: 4; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The tile's listeners: the press log and the two click counters.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let (b, log) = (find(dom, ".b"), find(dom, ".log"));
    for (ty, letter) in [("mouseup", 'u'), ("click", 'c')] {
        dom.add_event_listener(b, ty, ListenerOptions::default(), move |ctx| {
            let active = ctx.dom.node(b).matches(":active");
            let mut text = ctx.dom.text_content(log);
            text.push(letter);
            text.push(if active { '+' } else { '-' });
            ctx.dom.set_text_content(log, &text).unwrap();
        })
        .unwrap();
    }
    for (target, counter) in [(".u1", ".n1"), (".u2", ".n2")] {
        let (target, counter) = (find(dom, target), find(dom, counter));
        dom.add_event_listener(target, "click", ListenerOptions::default(), move |ctx| {
            let n: u32 = ctx
                .dom
                .text_content(counter)
                .parse()
                .expect("the counter holds a number");
            ctx.dom
                .set_text_content(counter, &(n + 1).to_string())
                .unwrap();
        })
        .unwrap();
    }
}
