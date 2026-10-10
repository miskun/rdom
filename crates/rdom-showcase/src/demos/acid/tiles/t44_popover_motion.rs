//! Tile 44 — popover entry and exit animations (CSS Transitions 2 §3
//! `@starting-style`, `transition-behavior`; CSS Position 4 §3.4
//! `overlay`; HTML §6.12; ACID-INTERACTIVE I16).
//!
//! Two `popovertarget` buttons for two popovers at the same fixed cells,
//! over a black tile and a maroon `z-index: 5` box: both fade in from
//! `@starting-style { opacity: 0 }`; `f` fades out under `transition:
//! opacity, display allow-discrete, overlay allow-discrete`, staying in
//! the top layer; `g` without `overlay`, leaving the top layer at once —
//! below the maroon box while it fades.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "44",
    title: "Popover entry and exit",
    class: "acid-t44",
    page: 12,
    x: 61,
    y: 1,
    w: 38,
    h: 4,
    markup: r#"
<button popovertarget="pf44">f</button> <button popovertarget="pg44">g</button><div class="cv"></div>
<div popover id="pf44" class="pf">fade</div><div popover id="pg44" class="pg">fade</div>
"#,
    css: r#"
.acid-t44 { background-color: rgb(0, 0, 0); }
.acid-t44 .cv {
  position: absolute; top: 2; left: 4; width: 2; height: 1; z-index: 5;
  background-color: rgb(128, 0, 0);
}
.acid-t44 [popover] {
  inset: auto; top: 3; left: 62; margin: 0; padding: 0; border: none;
  background-color: rgb(0, 0, 128); color: rgb(200, 200, 200);
}
.acid-t44 .pf {
  transition: opacity 100ms linear, display 100ms linear allow-discrete,
    overlay 100ms linear allow-discrete;
}
.acid-t44 .pg { transition: opacity 100ms linear, display 100ms linear allow-discrete; }
.acid-t44 [popover]:popover-open { opacity: 1; }
.acid-t44 [popover] { opacity: 0; }
@starting-style { .acid-t44 [popover]:popover-open { opacity: 0; } }
"#,
    late_css: "",
    setup: None,
    script: None,
};
