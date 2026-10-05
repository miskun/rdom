//! The computed `float`, `clear` and `display` (CSS 2.1 §9.7).

use crate::css_phase8::{el, lay_out};
use rdom_tui::layout::{Clear, Display, Float};
use rdom_tui::prelude::*;

fn computed(css: &str, tag: &str) -> (Float, Clear, Display) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let id = el(&mut dom, wrap, tag, "x");
    lay_out(&mut dom, css, 20, 4);
    let c = dom.node(id).computed().expect("cascaded");
    (c.float, c.clear, c.display)
}

/// §9.7: "Otherwise, if 'float' has a value other than 'none', the box is
/// floated and 'display' is set according to the table" — an inline box
/// becomes a block box; `clear` is not inherited and computes as written.
#[test]
fn a_float_is_blockified() {
    assert_eq!(
        computed(".x { float: left; clear: both }", "span"),
        (Float::Left, Clear::Both, Display::Block)
    );
    assert_eq!(
        computed(".x { float: inline-end; display: inline-block }", "span"),
        (Float::InlineEnd, Clear::None, Display::Block)
    );
    assert_eq!(
        computed(".x { }", "span"),
        (Float::None, Clear::None, Display::Inline)
    );
}

/// §9.7: "if 'position' has the value 'absolute' or 'fixed', the box is
/// absolutely positioned, the computed value of 'float' is 'none'".
#[test]
fn an_absolutely_positioned_box_does_not_float() {
    assert_eq!(
        computed(".x { float: right; position: absolute }", "div").0,
        Float::None
    );
    assert_eq!(
        computed(".x { float: right; position: relative }", "div").0,
        Float::Right
    );
}
