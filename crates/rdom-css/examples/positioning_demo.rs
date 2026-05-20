//! M2 demo — `position: absolute / fixed` + `z-index` + `opacity`.
//!
//! Three independent tiles. Each demonstrates one primitive in the
//! most obvious form — no kitchen-sink composition, no overlap
//! between tiles, every primitive labeled with the CSS that
//! produced it:
//!
//! 1. `position: absolute` anchored to a `position: relative` parent.
//!    A small tag sits in the parent's top-right corner; move the
//!    parent and the tag moves with it.
//! 2. `position: fixed` anchored to the viewport. A badge pinned to
//!    the viewport's top-right corner — ignores page flow.
//! 3. `z-index` + `opacity` stacking. Three translucent rectangles
//!    (red / green / blue, z = 1 / 2 / 3, each at `opacity: 0.6`).
//!    Higher z paints on top, and at each overlap region the
//!    painter's bg alpha-blends against the cell's actual existing
//!    bg (cell-level RMW). Distinct color bands at single / two-way
//!    / three-way overlaps make both z-order and the alpha-blend
//!    math visible.
//!
//! Press Ctrl-C to exit.
//!
//! Run with: `cargo run --example positioning_demo -p rdom-css`

use std::io;

use rdom_parser::parse_into;
use rdom_tui::prelude::*;
use rdom_tui::{extend_from_style_tags, seed_inline_styles};

const TEMPLATE: &str = r#"
<screen>
  <style>
    :root {
      --bg: #1f2123;
      --ink: #e0e0e0;
      --accent: #3d90ce;
      --dim: #707070;
      --red: #c04040;
      --green: #40a040;
      --blue: #4060c0;
    }

    page {
      display: block;
      color: var(--ink);
      padding: 0 1;
      gap: 1;
    }

    title   { font-weight: bold; color: var(--accent); height: 1; }
    hint    { color: var(--dim); height: 1; }
    heading { font-weight: bold; color: var(--accent); height: 1; }
    body    { color: var(--ink); height: 1; }

    /* Tile 1 — `position: absolute` against a `position: relative` parent. */
    relbox {
      position: relative;
      display: block;
      width: 56;
      height: 3;
      border: rounded;
      border-color: var(--dim);
      padding: 0 1;
    }
    abs-tag {
      position: absolute;
      top: 0;
      right: 0;
      width: 26;
      height: 1;
      background-color: var(--accent);
      color: black;
      padding: 0 1;
    }

    /* Tile 2 — `position: fixed` anchored to the viewport. */
    fixed-badge {
      position: fixed;
      top: 0;
      right: 0;
      width: 26;
      height: 1;
      background-color: var(--accent);
      color: black;
      padding: 0 1;
    }

    /* Tile 3 — `z-index` stacking + `opacity` cell-level alpha
       compositing. Three overlapping translucent cards. At each
       overlap, the upper card's bg alpha-blends against the actual
       cell bg below — so cells covered by red + green show olive
       (blend of green over already-blended red), cells covered by
       all three show the full stack blend. Demonstrates both
       z-order (higher z paints on top) and Phase 2 cell-level RMW
       compositing (the blend dst is the cell, not parent_bg). */
    zhost {
      position: relative;
      display: block;
      height: 5;
    }
    zcard1 {
      position: absolute;
      top: 0;  left: 0;
      width: 16; height: 3;
      padding: 0 1;
      background-color: var(--red);
      color: white;
      font-weight: bold;
      opacity: 0.6;
      z-index: 1;
    }
    zcard2 {
      position: absolute;
      top: 1;  left: 10;
      width: 16; height: 3;
      padding: 0 1;
      background-color: var(--green);
      color: white;
      font-weight: bold;
      opacity: 0.6;
      z-index: 2;
    }
    zcard3 {
      position: absolute;
      top: 2;  left: 20;
      width: 16; height: 3;
      padding: 0 1;
      background-color: var(--blue);
      color: white;
      font-weight: bold;
      opacity: 0.6;
      z-index: 3;
    }
  </style>

  <page>
    <title>rdom — position + z-index demo</title>
    <hint>Three independent tiles. Ctrl-C to exit.</hint>

    <fixed-badge>fixed: top:0 right:0 ↗</fixed-badge>

    <heading>1. position: absolute anchored to a position: relative parent</heading>
    <relbox>
      <body>this card is position: relative</body>
      <abs-tag>absolute: top:0 right:0</abs-tag>
    </relbox>

    <heading>2. position: fixed anchored to the viewport</heading>
    <body>↗ see the cyan badge pinned to the viewport's top-right corner.</body>

    <heading>3. z-index + opacity — translucent cards stack and alpha-blend at overlaps</heading>
    <zhost>
      <zcard1>z=1 (red)</zcard1>
      <zcard2>z=2 (green)</zcard2>
      <zcard3>z=3 (blue)</zcard3>
    </zhost>
  </page>
</screen>
"#;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, TEMPLATE, root).expect("template parses");

    let mut sheet = Stylesheet::new();
    let style_warnings = extend_from_style_tags(&dom, &mut sheet);
    let inline_warnings = seed_inline_styles(&mut dom);

    if !style_warnings.is_empty() {
        eprintln!("warnings from <style> blocks: {style_warnings:?}");
    }
    if !inline_warnings.is_empty() {
        eprintln!("warnings from inline style: {inline_warnings:?}");
    }

    let sheet = sheet
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .unwrap();

    App::new(dom, sheet)?.run()
}
