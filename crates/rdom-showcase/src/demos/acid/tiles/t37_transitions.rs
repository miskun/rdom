//! Tile 37 — transitions on the clock (CSS Transitions 1, CSS Easing 1,
//! Web Animations 1, CSS Values 5 `interpolate-size`; ACID-INTERACTIVE
//! I6).
//!
//! At rest nothing moves. Step I6's script adds `.on` to the tile and
//! opens its `details`; then, row 0: four swatches whose background goes
//! from black to `rgb(200, 0, 0)` under a `linear()` with a stop, a
//! `steps(2, jump-start)` after a 50ms delay, a negative delay and plain
//! `linear`, and a log the delayed swatch's `transitionrun` (`r`) and
//! `transitionstart` (`s`) listeners write; row 1 a flex row's `gap`,
//! row 2 a `padding-left`, rows 3 on a box's `height` beside a `details`
//! whose `::details-content` opens from `height: 0` to `auto` under
//! `interpolate-size: allow-keywords` — each over 100ms, laid out every
//! frame — and a last line pushed down by them.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "37",
    title: "Transitions on the clock",
    class: "acid-t37",
    page: 10,
    x: 1,
    y: 6,
    w: 58,
    h: 9,
    markup: r#"
<div class="sw"><div class="c1"></div><div class="c2"></div><div class="c3"></div><div class="c4"></div><span class="lg">log:</span></div>
<div class="gp"><span>a</span><span>b</span><span>c</span></div>
<div class="pd">pad</div>
<div class="bx"><div class="ht"></div><details class="dt"><summary>s</summary><div>1</div><div>2</div><div>3</div><div>4</div></details></div>
<div>end</div>
"#,
    css: r#"
.acid-t37 .sw, .acid-t37 .gp, .acid-t37 .bx { display: flex; }
.acid-t37 .sw { gap: 1; }
.acid-t37 .sw div { width: 4; height: 1; background-color: rgb(0, 0, 0); }
.acid-t37.on .sw div { background-color: rgb(200, 0, 0); }
.acid-t37 .c1 { transition: background-color 100ms linear(0, 0.75 50%, 1); }
.acid-t37 .c2 { transition: background-color 100ms steps(2, jump-start) 50ms; }
.acid-t37 .c3 { transition: background-color 100ms linear -50ms; }
.acid-t37 .c4 { transition: background-color 100ms linear; }
.acid-t37 .gp { gap: 0; transition: gap 100ms linear; }
.acid-t37.on .gp { gap: 4; }
.acid-t37 .pd { padding-left: 0; transition: padding-left 100ms linear; }
.acid-t37.on .pd { padding-left: 4; }
.acid-t37 .bx { gap: 1; align-items: flex-start; }
.acid-t37 .ht { width: 4; height: 1; background-color: rgb(0, 0, 128); transition: height 100ms linear; }
.acid-t37.on .ht { height: 5; }
.acid-t37 .dt { width: 6; interpolate-size: allow-keywords; }
.acid-t37 .dt::details-content {
  height: 0;
  overflow: clip;
  transition: height 100ms linear, content-visibility 100ms allow-discrete;
}
.acid-t37 .dt[open]::details-content { height: auto; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The delayed swatch's transition-event log.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let (c2, lg) = (find(dom, ".c2"), find(dom, ".lg"));
    for (ty, letter) in [("transitionrun", "r"), ("transitionstart", "s")] {
        dom.add_event_listener(c2, ty, ListenerOptions::default(), move |ctx| {
            let text = ctx.dom.text_content(lg) + letter;
            ctx.dom.set_text_content(lg, &text).unwrap();
        })
        .unwrap();
    }
}

/// Step I6's trigger: `.on` on the tile, the `details` opened.
pub fn start(dom: &mut TuiDom, tile: NodeId) {
    dom.add_class(tile, "on").unwrap();
    let dt = dom
        .query_selector_in(tile, ".dt")
        .expect("a valid selector")
        .expect("the tile holds it");
    dom.set_attribute(dt, "open", "").unwrap();
}
