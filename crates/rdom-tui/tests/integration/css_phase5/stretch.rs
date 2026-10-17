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

/// CSS Sizing 4 §3.1: `stretch` against an indefinite block size behaves
/// as the automatic size — for `min-height` `auto` (0 here, DIVERGENCES
/// §2), for `max-height` `none` — not the content height, which would
/// floor or cap a declared height at it (C16G-STRETCH-LEFTOVERS).
#[test]
fn a_stretch_min_or_max_height_against_an_indefinite_block_is_automatic() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let s = el(&mut dom, p, "div", "s");
    let t = dom.create_text_node("a b c d e");
    dom.append_child(s, t).unwrap();
    let m = el(&mut dom, p, "div", "m");
    let t = dom.create_text_node("x");
    dom.append_child(m, t).unwrap();
    lay_out(
        &mut dom,
        ".s { width: 1; height: 2; min-height: stretch; overflow: hidden }
         .m { height: 6; max-height: -webkit-fill-available }",
        20,
        20,
    );
    assert_eq!(rect(&dom, s).height, 2, "min-height: stretch is auto");
    assert_eq!(rect(&dom, m).height, 6, "max-height: stretch is none");
}

/// A `height: stretch` box's height is definite when its containing
/// block's is (CSS Sizing 4 §3.1, the stretch-fit size), so a percentage
/// or a `stretch` inside resolves against it — the iOS idiom `html {
/// height: -webkit-fill-available } body { min-height:
/// -webkit-fill-available }` fills the viewport (C16G-STRETCH-LEFTOVERS).
#[test]
fn a_stretched_ancestor_height_is_definite() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let html = el(&mut dom, root, "html", "");
    let body = el(&mut dom, html, "body", "");
    let half = el(&mut dom, body, "div", "half");
    lay_out(
        &mut dom,
        "html { height: -webkit-fill-available } body { margin: 0; min-height: -webkit-fill-available }
         .half { height: 50% }",
        20,
        12,
    );
    assert_eq!(rect(&dom, html).height, 12);
    assert_eq!(rect(&dom, body).height, 12, "body fills html");
    // `min-height` leaves the body's own height `auto`: indefinite.
    assert_eq!(rect(&dom, half).height, 0);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let s = el(&mut dom, p, "div", "s");
    let c = el(&mut dom, s, "div", "c");
    lay_out(
        &mut dom,
        ".p { height: 10 } .s { height: stretch; margin-top: 2 } .c { height: 50% }",
        20,
        12,
    );
    assert_eq!(rect(&dom, s).height, 8);
    assert_eq!(rect(&dom, c).height, 4, "50% of the stretched 8");
}

/// CSS Position 3 §4.1: an axis whose insets are both `auto` takes the
/// static position as its start inset (and 0 as its end), so `stretch`
/// fills from where the box would have been to the containing block's
/// end — not the whole containing block (C16G-STRETCH-LEFTOVERS).
#[test]
fn an_absolute_stretch_without_insets_starts_at_its_static_position() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let a = el(&mut dom, p, "div", "a");
    lay_out(
        &mut dom,
        ".p { position: relative; width: 20; height: 10; padding: 2 0 0 4 }
         .a { position: absolute; width: stretch; height: stretch }",
        30,
        14,
    );
    assert_eq!(xywh(rect(&dom, a)), (4, 2, 20, 10));
}
