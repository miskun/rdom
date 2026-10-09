//! Tile 28 — box longhands.
//!
//! Spec: CSS Backgrounds 3 §3.10 (the `background` shorthand: its final
//! layer's colour, image, position / size, repeat, attachment and two
//! boxes — origin then clip; the longhands after it reset each), §3.7 /
//! §3.8 (`background-clip: content-box` paints the content box only),
//! §4 (the border shorthands and longhands per side, `border-width`),
//! §5 (the four corner radii), §6.1 (`box-shadow` outside the border box,
//! moved by its offsets); CSS Box 4 §3 (`margin-trim: block` drops the
//! first child's top margin); CSS Sizing 3 (`min-height`); CSS 2.1 §11.1.2
//! (`clip: rect()` on an absolutely positioned box); CSS Containment 2 §4
//! / CSS Sizing 4 §5 (a `content-visibility: hidden` box is size
//! contained, sized by `contain-intrinsic-size` and its longhands — the
//! later ones win); CSS Conditional 5 §6.2 (`container-name` names the
//! container `@container nm` queries); CSS Color Adjust 1 §2 /
//! CSS Color 5 (`color-scheme: light` makes `light-dark()` take its first
//! arm). DIVERGENCES §2 "Background images parse and draw nothing" (only
//! the colour paints, clipped by the clip box), "A box shadow is a shade
//! of whole cells", §1 "A border corner takes one side's color" (the
//! heavier style, `double`, wins its corners; between equals the
//! horizontal side), "A rounded corner is one arc glyph", §2 "A clip path
//! clips whole cells" (the legacy `clip` by the same rule), §1 Images
//! (`mask*` and `mask-border*` kept, drawing nothing; `background-blend-
//! mode` inert), §1 sub-cell geometry (`rotate`, `scale`,
//! `transform-origin` inert — a stacking context, which nothing here
//! shows); `box-decoration-break` acts only at a fragment break and
//! `interpolate-size` only in a transition (none here).
//!
//! Derivation (one flex band, items 2 apart: x 0, 6, 11, 19, 25, 29, 34,
//! 40, 44, 50, 55):
//!
//! - `.bg`, 4 × 3 with padding 1: teal behind `bg` (x 1–2, row 1) only.
//! - `.sh`, 3 × 1 red, its blue shadow one cell right and down: x 7–9, row
//!   1.
//! - `.b4`, 6 × 4: top `solid` red, right `double` blue, bottom and left
//!   `solid` in the text colour — `┌` red (the top wins its equal left),
//!   `╖` and `╜` blue (`double` wins), `└` (the bottom wins).
//! - `.b5`, 4 wide, `solid`, a top-left and a bottom-right radius, empty:
//!   `╭──┐`, the `padding-bottom` row, `└──╯` (CSS 2.1 §9.4.2: no line
//!   box, no content row — a first draft gave it a `::before` space for
//!   one, which collapses away and makes none).
//! - `.mh`, 2 wide, `min-height: 3`: olive, rows 0–2.
//! - `.tr`, `margin-trim: block`: its paragraph's top margin trimmed — `t`
//!   on row 0, the box one row tall (maroon).
//! - `.cx`, `clip: rect(0, 2, 1, 0)`: `wxyz` cut to `wx`.
//! - `.mk`: teal `mk`, as if none of its mask and transform properties
//!   were there.
//! - `.cv`: `zz` skipped; its box 4 × 2 by `contain-intrinsic-width` /
//!   `-height` (navy).
//! - `cq` green (its container `nm` is wider than 0); `ls` green (the
//!   light arm).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "28",
    spec: &[
        "CSS Backgrounds 3 §3–§6; CSS Box 4 §3; CSS 2.1 §11.1.2",
        "CSS Containment 2 §4; CSS Conditional 5 §6.2; CSS Color Adjust 1 §2",
        "DIVERGENCES §1 corner, rounded corner, images; §2 background images, box shadow, clip path",
    ],
    legend: &[
        ('t', "bg #008080"),
        ('r', "bg #c80000"),
        ('u', "bg #0000ff"),
        ('R', "fg #ff0000"),
        ('b', "fg #0000ff"),
        ('o', "bg #808000"),
        ('m', "bg #800000"),
        ('n', "bg #000080"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|           ┌────╖  ╭──┐      t    wx    mk        cq   ls |
|......rrr..RRRRRb........oo..mmm........tt..nnnn..gg...gg.|
| bg        │    ║  │  │                                   |
|.tt....uuu......b........oo.................nnnn..........|
|           │    ║  └──╯                                   |
|................b........oo...............................|
|           └────╜                                         |
|................b.........................................|
"#,
};
