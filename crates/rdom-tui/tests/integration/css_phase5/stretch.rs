//! C15G-STRETCH — `stretch` (CSS Sizing 4 §3.1, with the aliases
//! `-webkit-fill-available` / `-moz-available`): the box's margin box
//! fills its containing block on that axis — "the size of the box … is
//! the stretch-fit size": the available space less the box's margins —
//! in block flow, floats, absolute positioning, flex and grid items.

use super::{el, lay_out, rect};
use rdom_tui::prelude::*;

fn xywh(r: rdom_tui::layout::LayoutRect) -> (i32, i32, u16, u16) {
    (r.x, r.y, r.width, r.height)
}

/// A float or an inline-block shrinks to fit; `width: stretch` fills the
/// line less its margins instead (`-webkit-fill-available` the same), and
/// `box-sizing` does not change the outer size.
#[test]
fn a_stretch_width_fills_the_containing_block_less_its_margins() {
    for css in [
        ".s { float: left; width: stretch; margin: 0 3 0 2 }",
        ".s { display: inline-block; width: -webkit-fill-available; margin: 0 3 0 2 }",
        ".s { float: left; width: -moz-available; margin: 0 3 0 2; padding: 0 4; box-sizing: content-box }",
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let s = el(&mut dom, root, "div", "s");
        let t = dom.create_text_node("x");
        dom.append_child(s, t).unwrap();
        lay_out(&mut dom, css, 20, 4);
        assert_eq!(rect(&dom, s).width, 15, "{css}");
    }
}

/// On the block axis `stretch` is the definite containing block height
/// less the margins; against an indefinite one it behaves as `auto`.
#[test]
fn a_stretch_height_fills_a_definite_containing_block() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let s = el(&mut dom, p, "div", "s");
    lay_out(
        &mut dom,
        ".p { height: 10 } .s { height: stretch; margin: 1 0 2 0 }",
        20,
        12,
    );
    assert_eq!(rect(&dom, s).height, 7);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let s = el(&mut dom, p, "div", "s");
    let t = dom.create_text_node("x");
    dom.append_child(s, t).unwrap();
    lay_out(&mut dom, ".s { height: stretch }", 20, 12);
    assert_eq!(rect(&dom, s).height, 1, "indefinite: auto");
}

/// An absolutely positioned box's `stretch` fills its inset-modified
/// containing block less its margins, where `auto` would shrink to fit.
#[test]
fn an_absolute_stretch_fills_the_inset_modified_containing_block() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    let t = dom.create_text_node("x");
    dom.append_child(a, t).unwrap();
    lay_out(
        &mut dom,
        ".a { position: absolute; top: 1; left: 3; width: stretch; height: stretch; margin: 0 1 1 0 }",
        20,
        10,
    );
    assert_eq!(xywh(rect(&dom, a)), (3, 1, 16, 8));
}

/// A flex item's `width: stretch` is its container's width less its
/// margins (its flex base size), where `auto` is its content; a grid
/// item's fills its area less its margins.
#[test]
fn flex_and_grid_items_stretch() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = el(&mut dom, f, "div", "i");
    let t = dom.create_text_node("x");
    dom.append_child(i, t).unwrap();
    lay_out(
        &mut dom,
        ".f { display: flex; width: 20 } .i { width: stretch; margin: 0 2; flex-shrink: 0 }",
        30,
        4,
    );
    assert_eq!(xywh(rect(&dom, i)), (2, 0, 16, 1));
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let i = el(&mut dom, g, "div", "i");
    let t = dom.create_text_node("x");
    dom.append_child(i, t).unwrap();
    lay_out(
        &mut dom,
        ".g { display: grid; grid-template-columns: 12 8; justify-items: start }
         .i { width: stretch; margin: 0 1 }",
        30,
        4,
    );
    assert_eq!(xywh(rect(&dom, i)), (1, 0, 10, 1));
}
