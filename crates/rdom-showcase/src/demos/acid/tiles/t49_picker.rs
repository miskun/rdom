//! Tile 49 — a picker and an article that follow the terminal's height
//! (CSS Anchor Positioning 1 §3.1, §4; CSS Multi-column 1 §3.4;
//! ACID-INTERACTIVE I21).
//!
//! An article `calc(100vh - 24)` wide under `column-width: 8`, and a
//! `popovertarget` button anchoring its popover by `position-area: bottom
//! span-right` with `position-try-fallbacks: flip-block`, on the page's
//! rows 37–45 — so a terminal four rows shorter narrows the article and
//! leaves the popover no room below its button.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "49",
    title: "Picker and columns by height",
    class: "acid-t49",
    page: 7,
    x: 1,
    y: 37,
    w: 58,
    h: 9,
    markup: r#"
<div class="ar"><div>c1</div><div>c2</div><div>c3</div><div>c4</div><div>c5</div><div>c6</div></div><button popovertarget="pk49" class="pb">pick</button><div popover id="pk49" class="pk"><div>alpha</div><div>beta</div></div>
"#,
    css: r#"
.acid-t49 .ar { column-width: 8; column-gap: 1; width: calc(100vh - 24); }
.acid-t49 .pb { position: absolute; left: 30; top: 5; anchor-name: --acid-pick; }
.acid-t49 .pk {
  inset: auto; margin: 0; padding: 0;
  position-anchor: --acid-pick; position-area: bottom span-right;
  position-try-fallbacks: flip-block;
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
