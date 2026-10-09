//! Tile 31 — animations and transitions in one frame (CSS Animations 1 /
//! 2, Web Animations 1, Scroll-driven Animations 1, CSS Transitions 1 / 2,
//! CSS Properties and Values 1, CSS Cascade 6 `@scope`; ACID-COVERAGE).
//!
//! The page's clock stands at 0, so every animation and transition here
//! shows a value its negative delay or its timeline fixes: a paused
//! `animation` shorthand half-way through; the animation longhands in a
//! reversed second iteration; `animation-composition: add`; a
//! scroll-driven animation at its scroller's start, with the scroll and
//! view timeline properties; a transition the load script starts with a
//! negative delay; an `@starting-style` entry; an `@property` length's
//! initial value; and an `@scope` with a lower bound.

use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "31",
    title: "Animations and transitions at time 0",
    class: "acid-t31",
    page: 8,
    x: 61,
    y: 14,
    w: 58,
    h: 10,
    markup: r#"
<div class="a1"></div><div class="a2"></div><div class="a3"></div><div class="ts"><div class="sc"><div class="bar"></div><div class="v"></div><div class="fill"></div></div></div><div class="t1"></div><div class="st"></div><div class="pr"></div><div class="sc1"><p class="w">in</p><div class="lim"><p class="w">out</p></div></div>
"#,
    css: r#"
@keyframes acid-t31-grow { from { width: 0; } to { width: 10; } }
@keyframes acid-t31-add { from { width: 0; } to { width: 4; } }
@keyframes acid-t31-bar { from { width: 3; } to { width: 10; } }
@property --acid-t31-w { syntax: '<length>'; inherits: false; initial-value: 3; }
.acid-t31 .a1 { height: 1; background-color: rgb(0, 128, 128); animation: acid-t31-grow 10s linear -5s both paused; }
.acid-t31 .a2 {
  height: 1; background-color: rgb(0, 0, 128);
  animation-name: acid-t31-grow; animation-duration: 10s; animation-timing-function: linear;
  animation-delay: -12s; animation-iteration-count: 2; animation-direction: alternate;
  animation-fill-mode: both; animation-play-state: paused; animation-composition: replace;
}
.acid-t31 .a3 {
  width: 2; height: 1; background-color: rgb(128, 128, 0);
  animation: acid-t31-add 10s linear -5s paused; animation-composition: add;
}
.acid-t31 .ts { timeline-scope: --acid-t31-v; }
.acid-t31 .sc {
  width: 12; height: 2; overflow: auto; scrollbar-width: none;
  scroll-timeline: --acid-t31-t block; scroll-timeline-name: --acid-t31-t; scroll-timeline-axis: block;
}
.acid-t31 .bar {
  height: 1; background-color: rgb(0, 128, 128);
  animation: acid-t31-bar linear both; animation-timeline: --acid-t31-t;
  animation-range: normal; animation-range-start: normal; animation-range-end: normal;
}
.acid-t31 .v {
  height: 1;
  view-timeline: --acid-t31-v block; view-timeline-name: --acid-t31-v;
  view-timeline-axis: block; view-timeline-inset: auto;
}
.acid-t31 .fill { height: 3; }
.acid-t31 .t1 {
  width: 2; height: 1; background-color: rgb(128, 0, 0);
  transition: width 10s linear -5s;
  transition-property: width; transition-duration: 10s; transition-timing-function: linear;
  transition-delay: -5s; transition-behavior: normal;
}
.acid-t31 .t1.go { width: 10; }
.acid-t31 .st { width: 6; height: 1; background-color: rgb(0, 128, 128); transition: width 10s linear -5s; }
@starting-style { .acid-t31 .st { width: 2; } }
.acid-t31 .pr { width: var(--acid-t31-w); height: 1; background-color: rgb(0, 0, 128); }
@scope (.acid-t31 .sc1) to (.lim) { .w { color: rgb(0, 160, 0); } }
"#,
    late_css: "",
    setup: None,
    script: Some(start_transition),
};

/// The load handler: add `go` to `.t1`, starting its width transition.
fn start_transition(dom: &mut TuiDom, tile: NodeId) {
    let t1 = dom
        .query_selector_in(tile, ".t1")
        .expect("a valid selector")
        .expect("the tile holds .t1");
    dom.add_class(t1, "go").expect("an element");
}
