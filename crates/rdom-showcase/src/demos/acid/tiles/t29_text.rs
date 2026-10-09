//! Tile 29 — text and font longhands (CSS Fonts 4, CSS Text 3 / 4, CSS
//! Text Decoration 4, CSS Overflow 4 §4, CSS Lists 3, CSS UI 4;
//! ACID-COVERAGE).
//!
//! The font properties a terminal keeps and does not draw; the text
//! decoration longhands; `word-spacing`; `white-space-collapse` and
//! `text-wrap-mode`; `word-wrap`, `line-break: anywhere`, `hyphens`;
//! `text-align-all`, `text-justify`, `text-wrap-style`; the line-clamp
//! longhands (`max-lines`, `block-ellipsis`, `continue`); the list
//! longhands; `pointer-events`, `cursor`, `resize` and
//! `-webkit-appearance`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "29",
    title: "Text and font longhands",
    class: "acid-t29",
    page: 8,
    x: 61,
    y: 7,
    w: 58,
    h: 5,
    markup: r#"
<div class="band"><div class="fo">font</div><div class="dl">deco</div><div class="ws">a b c</div><div class="wc">x  y</div><div class="ww">abcdefgh</div><div class="lb">abcdef</div><div class="hy">ab&shy;cd</div><div class="ta">ab cd</div></div>
<div class="band"><div class="lc">aa bb cc</div><ul class="ls"><li>x</li></ul><div class="pe"></div><button class="wa">B</button></div>
"#,
    css: r#"
.acid-t29 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t29 .fo {
  font-size: 20px; font-family: monospace; font-stretch: 50%; font-width: normal;
  font-variant: small-caps;
}
.acid-t29 .dl {
  text-decoration-line: underline; text-decoration-style: wavy;
  text-decoration-color: rgb(255, 0, 0); text-decoration-thickness: 2px;
  text-underline-offset: 1px; text-underline-position: under; text-decoration-skip-ink: none;
}
.acid-t29 .ws { word-spacing: 2; }
.acid-t29 .wc { white-space-collapse: preserve; text-wrap-mode: nowrap; }
.acid-t29 .ww { width: 4; word-wrap: break-word; }
.acid-t29 .lb { width: 3; line-break: anywhere; }
.acid-t29 .hy { width: 3; hyphens: auto; }
.acid-t29 .ta { width: 7; text-align-all: right; text-justify: inter-word; text-wrap-style: pretty; }
.acid-t29 .lc { width: 6; max-lines: 1; block-ellipsis: auto; continue: discard; }
.acid-t29 .ls { list-style: square inside; list-style-image: none; marker-side: match-self; }
.acid-t29 .pe {
  width: 3; height: 2; overflow: auto; resize: both;
  pointer-events: none; cursor: grab;
}
.acid-t29 .wa { -webkit-appearance: none; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
