//! Tile 24 — filters, blending and clipping (Filter Effects 1 / 2,
//! Compositing and Blending 1, CSS Masking 1).
//!
//! `invert(1)` on a black box with white text; `grayscale(1)`,
//! `sepia(1)`, `hue-rotate(180deg)` and `contrast(0.5)` swatches;
//! `filter: opacity(0.5)` beside `opacity: 0.5`; a `drop-shadow()` under
//! a badge; a `mask` that draws nothing; `mix-blend-mode` `multiply`,
//! `difference` and `luminosity` over an orange box, and `multiply` inside
//! an `isolation: isolate` group without and with a background; a
//! `backdrop-filter: invert(1)` strip with a translucent background over
//! text; `clip-path` `inset() round`, `circle()` and `polygon()`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "24",
    title: "Filters, blending & clipping",
    class: "acid-t24",
    page: 6,
    x: 61,
    y: 12,
    w: 58,
    h: 10,
    markup: r#"
<div class="band"><div class="sw inv">inv</div><div class="sw gray"></div><div class="sw sep"></div><div class="sw hue"></div><div class="sw con"></div><div class="sw fop"></div><div class="sw op"></div><div class="drop"></div><div class="sw msk">msk</div></div>
<div class="band"><div class="org"><div class="bl mul"></div><div class="bl dif"></div><div class="bl lum"></div><div class="iso1"><div class="bl mul"></div></div><div class="iso2"><div class="bl mul"></div></div></div></div>
<div class="band"><div class="bfw">abcdef<div class="bf"></div></div><div class="ins"></div><div class="cir"></div><div class="pol"></div></div>
"#,
    css: r#"
.acid-t24 .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t24 .sw { width: 4; height: 1; }
.acid-t24 .inv { filter: invert(1); background-color: rgb(0, 0, 0); color: rgb(255, 255, 255); }
.acid-t24 .gray { filter: grayscale(1); background-color: rgb(255, 0, 0); }
.acid-t24 .sep { filter: sepia(1); background-color: rgb(100, 100, 100); }
.acid-t24 .hue { filter: hue-rotate(180deg); background-color: rgb(255, 0, 0); }
.acid-t24 .con { filter: contrast(0.5); background-color: rgb(200, 200, 200); }
.acid-t24 .fop { filter: opacity(0.5); background-color: rgb(200, 0, 0); }
.acid-t24 .op { opacity: 0.5; background-color: rgb(200, 0, 0); }
.acid-t24 .drop {
  width: 3; height: 1; background-color: rgb(200, 0, 0);
  filter: drop-shadow(rgb(0 0 255) 1 1);
}
.acid-t24 .msk { mask: url(m.svg); background-color: rgb(0, 128, 128); }
.acid-t24 .org { display: flex; gap: 1; padding: 0 1; background-color: rgb(255, 165, 0); }
.acid-t24 .bl { width: 3; height: 1; }
.acid-t24 .mul { mix-blend-mode: multiply; background-color: rgb(128, 128, 128); }
.acid-t24 .dif { mix-blend-mode: difference; background-color: rgb(0, 0, 255); }
.acid-t24 .lum { mix-blend-mode: luminosity; background-color: rgb(0, 0, 255); }
.acid-t24 .iso1 { isolation: isolate; }
.acid-t24 .iso2 { isolation: isolate; background-color: rgb(0, 255, 0); }
.acid-t24 .bfw { position: relative; width: 6; }
.acid-t24 .bf {
  position: absolute; left: 1; top: 0; width: 3; height: 1;
  backdrop-filter: invert(1); background-color: rgb(0 0 100 / 60%);
}
.acid-t24 .ins { width: 10; height: 6; clip-path: inset(0 round 3); background-color: rgb(0, 128, 128); }
.acid-t24 .cir { width: 6; height: 4; clip-path: circle(2 at 3 2); background-color: rgb(0, 0, 128); }
.acid-t24 .pol { width: 6; height: 3; clip-path: polygon(0 0, 6 0, 0 3); background-color: rgb(128, 128, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
