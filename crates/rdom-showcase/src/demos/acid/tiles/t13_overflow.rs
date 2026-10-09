//! Tile 13 — overflow and scrollbars (CSS Overflow 3 / 4, CSS Scrollbars
//! 1, CSS Overflow 3 §3 `scrollbar-gutter`).
//!
//! Clipping by `overflow: hidden` (text and a positioned child); an
//! `overflow: auto` box scrolled by its load script, with a thin,
//! coloured bar; `overflow-x: clip` with `overflow-clip-margin`;
//! `text-overflow: ellipsis` in an `ltr` and an `rtl` line; `line-clamp`
//! and its autoprefixed `-webkit-box` form; an absolutely positioned box
//! past the content counting in the scrollable overflow, its bar styled by
//! `::scrollbar` / `::scrollbar-thumb`; `scrollbar-gutter: stable` and
//! `stable both-edges`; a scroll container inside a scroll container whose
//! overflow stays its own.

use rdom_tui::{NodeId, TuiAccessorsMut, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "13",
    title: "Overflow & scrollbars",
    class: "acid-t13",
    page: 2,
    x: 0,
    y: 30,
    w: 58,
    h: 9,
    markup: r#"
<div class="band"><div class="ovh">aaaa bbbbbbbbbbbb<b class="kid"></b></div><div class="sa"><p>1</p><p>2</p><p>3</p><p>4</p><p>5</p><p>6</p><p>7</p><p>8</p></div><div class="oc">cccccccccc dd ee ff</div><div class="dcol"><p class="te">abcdefghijk</p><p class="te rt">abcdefghijk</p></div><div class="ecol"><p class="lc">aa bb cc dd ee ff</p><p class="wk">aa bb cc dd ee ff gg</p></div><div class="ap">x<b class="far">z</b></div><div class="sg">ab cd</div><div class="sg2">ab cd</div></div>
<div class="band"><div class="ou">12345678<div class="in"><p>a</p><p>b</p><p>c</p><p>d</p></div></div></div>
"#,
    css: r#"
.acid-t13 .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t13 b { font-weight: normal; }
.acid-t13 .ovh { position: relative; width: 8; height: 2; overflow: hidden; }
.acid-t13 .kid {
  position: absolute; left: 6; top: 0; width: 6; height: 1;
  background-color: rgb(0, 128, 128);
}
.acid-t13 .sa {
  width: 6; height: 4; overflow: auto;
  scrollbar-width: thin; scrollbar-color: rgb(255, 255, 0) rgb(0, 0, 128);
}
.acid-t13 .oc {
  width: 6; height: 2; overflow-x: clip; overflow-y: visible; overflow-clip-margin: 1;
}
.acid-t13 .dcol { width: 8; }
.acid-t13 .te { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.acid-t13 .rt { direction: rtl; }
.acid-t13 .ecol { width: 6; }
.acid-t13 .lc { line-clamp: 2; }
.acid-t13 .wk {
  display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3;
}
.acid-t13 .ap { position: relative; width: 5; height: 3; overflow: auto; }
.acid-t13 .far { position: absolute; top: 8; left: 0; height: 1; }
.acid-t13 .ap::scrollbar, .acid-t13 .in::scrollbar { content: "░"; color: rgb(0, 160, 0); }
.acid-t13 .ap::scrollbar-thumb, .acid-t13 .in::scrollbar-thumb { content: "█"; color: rgb(0, 160, 0); }
.acid-t13 .sg { width: 5; height: 2; overflow: auto; scrollbar-gutter: stable; }
.acid-t13 .sg2 { width: 5; height: 2; overflow: auto; scrollbar-gutter: stable both-edges; }
.acid-t13 .ou { width: 8; height: 3; overflow: auto; }
.acid-t13 .in { height: 2; overflow: auto; }
"#,
    late_css: "",
    setup: None,
    script: Some(scroll_sa),
};

/// The page's load handler: scroll `.sa` two rows down
/// (`element.scrollTop = 2`).
fn scroll_sa(dom: &mut TuiDom, tile: NodeId) {
    let sa = dom
        .query_selector_in(tile, ".sa")
        .expect("a valid selector")
        .expect("the tile holds .sa");
    dom.node_mut(sa)
        .set_scroll_top(2)
        .expect("a scroll container");
}
