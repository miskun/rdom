//! Tile 54 — a monolithic box moving whole, and cloned decorations
//! (CSS Multi-column 1 §7; CSS Fragmentation 3 §4.1, §5.4;
//! ACID-STATIC-REST, tile 25's left-out cases).
//!
//! Two 17-wide, two-column articles under `column-fill: auto`: the first,
//! 3 rows tall, holds an `overflow: hidden` box of two rows after two
//! lines; the second, 4 rows tall, a bordered box of four lines under
//! `box-decoration-break: clone`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "54",
    title: "Multicol: whole box, cloned border",
    class: "acid-t54",
    page: 13,
    x: 41,
    y: 10,
    w: 38,
    h: 4,
    markup: r#"
<div class="mc ma"><div>a1</div><div>a2</div><div class="oh"><div>H1</div><div>H2</div></div><div>a3</div></div><div class="mc mb"><div class="q"><div>q1</div><div>q2</div><div>q3</div><div>q4</div></div></div>
"#,
    css: r#"
.acid-t54 .mc {
  display: inline-block; vertical-align: top; margin-right: 2;
  columns: 2; column-gap: 1; column-fill: auto; width: 17;
}
.acid-t54 .ma { height: 3; }
.acid-t54 .mb { height: 4; }
.acid-t54 .oh { overflow: hidden; height: 2; background-color: rgb(0, 0, 128); }
.acid-t54 .q { border: solid; box-decoration-break: clone; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
