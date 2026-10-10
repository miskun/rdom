//! Tile 39 — smooth scrolling and `scrollIntoView` (CSSOM View §4.1,
//! §5.1, §12.1; ACID-INTERACTIVE I8).
//!
//! An outer scroll container holding a line, an inner scroll container
//! of twelve lines under `scroll-behavior: smooth` (navy), and five more
//! lines; neither draws a scrollbar (`scrollbar-width: none`), both
//! scroll. Step I8 scrolls the inner box by script and brings one of its
//! lines into view.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "39",
    title: "Smooth scroll, scrollIntoView",
    class: "acid-t39",
    page: 10,
    x: 61,
    y: 9,
    w: 38,
    h: 5,
    markup: r#"
<div class="out"><div>top</div><div class="in"><div>a0</div><div>a1</div><div class="tg">a2</div><div>a3</div><div>a4</div><div>a5</div><div>a6</div><div>a7</div><div>a8</div><div>a9</div><div>a10</div><div>a11</div></div><div>o2</div><div>o3</div><div>o4</div><div>o5</div><div>o6</div></div>
"#,
    css: r#"
.acid-t39 .out { width: 10; height: 5; overflow: auto; scrollbar-width: none; }
.acid-t39 .in {
  width: 6; height: 3; overflow: auto; scrollbar-width: none;
  scroll-behavior: smooth; background-color: rgb(0, 0, 128);
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
