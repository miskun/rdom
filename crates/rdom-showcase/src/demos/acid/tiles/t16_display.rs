//! Tile 16 — display and visibility (CSS Display 3, CSS 2.1 §11.2,
//! Flexbox 1 §4.4, HTML §15.5.20 / the `hidden` attribute).
//!
//! `display: none` and the `hidden` attribute taking no space beside
//! `visibility: hidden` keeping its cells; the multi-keyword syntax
//! (`inline flow-root`, `block flow`, `block flex`) beside `inline-flex`;
//! a block among inline content; `display: contents` children joining a
//! flex container, a blockified `<span>` taking a `width`; `visibility:
//! collapse` on a flex item (a strut: no main size, its cross size kept)
//! and on a table row (removed); `flow-root` keeping a child's margin
//! inside where a plain block lets it collapse through; a closed and an
//! open `<details>`; an `:empty` box.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "16",
    title: "Display & visibility",
    class: "acid-t16",
    page: 3,
    x: 1,
    y: 16,
    w: 58,
    h: 16,
    markup: r#"
<div>a<span class="dn">X</span>b <span class="vh">XX</span>c <span hidden>H</span>d</div>
<div>x<div class="ib">IB</div>y<span class="if">FL</span>z</div>
<div><span class="blk">bl</span><span>in</span></div>
<div class="fx"><div class="dc"><span>p</span><span>q</span></div><span class="w3">r</span><span class="vc">s</span><span>t</span></div>
<div class="nr"><p class="m1">u</p></div><div class="fr"><p class="m1">v</p></div>
<details><summary>S</summary>c</details><details open><summary>O</summary>k</details>
<div class="dn">gone</div>
<div class="em"></div>
<table class="tc"><tr><td>1</td></tr><tr class="vc"><td>2</td></tr><tr><td>3</td></tr></table>
"#,
    css: r#"
.acid-t16 .dn { display: none; }
.acid-t16 .vh { visibility: hidden; }
.acid-t16 .ib { display: inline flow-root; background-color: rgb(0, 128, 128); }
.acid-t16 .if { display: inline-flex; background-color: rgb(0, 0, 128); }
.acid-t16 .blk { display: block flow; }
.acid-t16 .fx { display: block flex; gap: 1; }
.acid-t16 .dc { display: contents; }
.acid-t16 .w3 { width: 3; background-color: rgb(128, 128, 0); }
.acid-t16 .vc { visibility: collapse; }
.acid-t16 .fx .vc { height: 2; }
.acid-t16 .m1 { margin-top: 1; }
.acid-t16 .nr { background-color: rgb(128, 0, 0); }
.acid-t16 .fr { display: flow-root; background-color: rgb(0, 0, 128); }
.acid-t16 .em:empty { width: 2; height: 1; background-color: rgb(0, 128, 128); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
