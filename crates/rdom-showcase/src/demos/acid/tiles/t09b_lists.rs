//! Tile 9b — lists and markers (CSS Lists 3 §3–§4, HTML §4.4.5–§4.4.8,
//! §15.3.8).
//!
//! Outside markers hanging in the list's four cells of padding — at the
//! tile's left edge, where a marker wider than the padding is cut, and
//! beside a margin, where it is whole; wrapped lines under the text, an
//! empty item, an `inside` marker; three nested bullet levels with their
//! markers on one line; an `rtl` list; `start`, `reversed`, `value`,
//! `type`; `::marker` colour and `content`; a list-item `::before` with
//! its own `::marker`; an `inline list-item`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "9b",
    title: "Lists & markers",
    class: "acid-t9b",
    page: 2,
    x: 0,
    y: 20,
    w: 58,
    h: 8,
    markup: r#"
<div class="band"><div class="ca"><ol start="9"><li>a</li><li>b</li></ol><ol type="I" start="3"><li>c</li></ol><ol class="ml" type="I" start="3"><li>c</li></ol></div><ol class="ob"><li><p>aa bb cc dd</p></li><li></li><li class="in">dd ee</li></ol><div class="cc"><ul><li><ul><li><ul><li>x</li></ul></li></ul></li></ul><ul dir="rtl"><li>ab</li></ul></div><div class="cd"><ol reversed><li>a</li><li>b</li></ol><ol><li value="7">b</li><li>c</li></ol><ol type="a"><li>x</li></ol></div></div>
<div class="band"><ul class="mk"><li>red</li><li class="mc">txt</li></ul><div class="lb">body</div><p class="ili">x <span class="li2">yy</span></p></div>
"#,
    css: r#"
.acid-t9b .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t9b .ca { width: 14; }
.acid-t9b .ml { margin-left: 2; }
.acid-t9b .ob { width: 8; }
.acid-t9b .in { list-style-position: inside; }
.acid-t9b .cc { width: 14; }
.acid-t9b .cd { width: 13; }
.acid-t9b .mk { width: 6; }
.acid-t9b .mk li::marker { color: rgb(192, 0, 0); }
.acid-t9b .mc::marker { content: "→ "; }
.acid-t9b .mk li::marker:hover { color: rgb(0, 0, 192); }
.acid-t9b .in::marker:hover { color: rgb(0, 0, 192); }
.acid-t9b .lb { width: 6; padding-left: 4; }
.acid-t9b .lb::before { content: "B"; display: list-item; list-style-type: square; }
.acid-t9b .lb::before::marker { color: rgb(0, 160, 0); }
.acid-t9b .ili { width: 10; }
.acid-t9b .li2 { display: inline list-item; list-style-type: disc; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
