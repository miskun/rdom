//! Floats in paint and hit-testing (CSS 2.1 Appendix E step 5: floats
//! paint after the block backgrounds of step 4, each atomically).

use super::place::boxed;
use crate::css_phase8::{el, paint};
use rdom_tui::prelude::*;
use rdom_tui::render::Color;

const CSS: &str = ".c { width: 10 } .f { float: left; width: 2; height: 1 } \
                   .p { background-color: #0000ff }";

/// `.c` holding the float `XY` and then a paragraph with a background.
fn tree() -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let f = boxed(&mut dom, c, "f", "XY");
    let p = boxed(&mut dom, c, "p", "aa");
    (dom, f, p)
}

/// Appendix E: the paragraph is a block box under the float (§9.5:
/// floats do not move block boxes), its background painted first (step
/// 4), the float after it (step 5) — the float's text shows on the
/// paragraph's background, and the paragraph's line beside it.
#[test]
fn a_float_paints_over_the_next_blocks_background() {
    let (mut dom, _, _) = tree();
    let buf = paint(&mut dom, CSS, 10, 1);
    let cell = |x| buf.cell(x, 0).unwrap();
    assert_eq!(
        (cell(0).symbol(), cell(2).symbol(), cell(3).symbol()),
        ("X", "a", "a")
    );
    assert_eq!(
        cell(0).bg,
        Color::Rgb(0, 0, 255),
        "the paragraph's background"
    );
}

/// Hit-testing walks paint order backward: the float above the block it
/// overlaps, the paragraph beside it.
#[test]
fn a_float_is_hit_above_the_block_it_overlaps() {
    let (mut dom, f, p) = tree();
    paint(&mut dom, CSS, 10, 1);
    assert_eq!(dom.element_from_point(1, 0).map(|n| n.id()), Some(f));
    assert_eq!(dom.element_from_point(5, 0).map(|n| n.id()), Some(p));
}
