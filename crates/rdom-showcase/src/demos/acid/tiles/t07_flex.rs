//! Tile 7 — flex layout (CSS Flexible Box 1 §5–§9, CSS Box Alignment 3).
//!
//! One small flex container per row (or rows), stacked in block flow:
//! `flex` grow and shrink with a min-content clamp, `row-reverse`,
//! `justify-content`, `auto` margins, `align-self` (stretch, center,
//! flex-end, baseline), `column-reverse`, `flex-wrap` with `order` and
//! both gaps, `wrap-reverse`, and anonymous and `::before` / `::after`
//! items. Items are coloured by DOM position (navy, teal, purple) so each
//! item's extent shows.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "7",
    title: "Flex layout",
    class: "acid-t7",
    page: 1,
    x: 1,
    y: 24,
    w: 58,
    h: 18,
    markup: r#"
<div class="f1"><div class="ga">a</div><div class="gb">b</div><div class="gc">c</div></div>
<div class="f2"><div class="sa">aaaa bbbb</div><div class="sb">longword</div></div>
<div class="f3"><div>1</div><div>2</div><div>3</div></div>
<div class="f4"><div>1</div><div>2</div><div>3</div></div>
<div class="f4c"><div>c</div><div>c</div><div>c</div></div>
<div class="f5"><div>a</div><div class="am">b</div><div>c</div></div>
<div class="f6"><div class="st">a</div><div class="ce">b</div><div class="en">c</div><div class="bl pt">e</div><div class="bl">f</div></div>
<div class="f7"><div>1</div><div>2</div><div>3</div></div>
<div class="f8"><div>1</div><div>2</div><div class="o">3</div></div>
<div class="f8r"><div>1</div><div>2</div><div>3</div></div>
<div class="f9">anon<b>el</b></div>
"#,
    css: r#"
.acid-t7 > div { display: flex; }
.acid-t7 > div > div:nth-child(3n+1) { background-color: rgb(0, 0, 128); }
.acid-t7 > div > div:nth-child(3n+2) { background-color: rgb(0, 128, 128); }
.acid-t7 > div > div:nth-child(3n) { background-color: rgb(128, 0, 128); }
.acid-t7 .f1 { width: 31; }
.acid-t7 .ga { flex: 1 1 4ch; }
.acid-t7 .gb { flex: 2 1 4ch; }
.acid-t7 .gc { flex: none; width: 5; }
.acid-t7 .f2 { width: 20; }
.acid-t7 .sa { flex: 0 1 15ch; }
.acid-t7 .sb { flex: 0 4 15ch; }
.acid-t7 .f3 { width: 30; flex-direction: row-reverse; }
.acid-t7 .f3 > div { width: 3; }
.acid-t7 .f4 { width: 20; justify-content: space-between; }
.acid-t7 .f4c { width: 21; justify-content: center; }
.acid-t7 .f5 { width: 20; }
.acid-t7 .f4 > div, .acid-t7 .f4c > div, .acid-t7 .f5 > div { width: 2; }
.acid-t7 .am { margin: 0 auto; }
.acid-t7 .f6 { width: 30; height: 3; align-items: flex-start; }
.acid-t7 .f6 > div { width: 3; }
.acid-t7 .st { align-self: stretch; }
.acid-t7 .ce { align-self: center; }
.acid-t7 .en { align-self: flex-end; }
.acid-t7 .bl { align-self: baseline; }
.acid-t7 .pt { padding-top: 1; }
.acid-t7 .f7 { width: 4; flex-direction: column-reverse; }
.acid-t7 .f8 { width: 14; flex-wrap: wrap; column-gap: 2; row-gap: 1; }
.acid-t7 .f8 > div, .acid-t7 .f8r > div { width: 6; }
.acid-t7 .o { order: -1; }
.acid-t7 .f8r { width: 14; flex-wrap: wrap-reverse; column-gap: 2; }
.acid-t7 .f9 { gap: 1; }
.acid-t7 .f9::before { content: "B"; }
.acid-t7 .f9::after { content: "A"; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
