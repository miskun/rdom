//! C7-GRID-PLACE — absolutely positioned boxes in a grid (CSS Grid 2
//! §9.1): a grid container that is their containing block gives each
//! the grid area its placement properties name; an `auto` line — or a
//! line the grid does not have — is the containing block's edge.

use super::{el, lay_out, rect};
use rdom_tui::TuiDom;

/// The rect of an absolutely positioned `.abs` (with `inset: 0`, so it
/// fills its containing block) in a 20-wide positioned grid of columns
/// `2 3 4` and rows `1 2`, placed by `placement`.
fn placed(placement: &str) -> (i32, i32, u16, u16) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let abs = el(&mut dom, g, "div", "abs");
    lay_out(
        &mut dom,
        &format!(
            ".g {{ display: grid; position: relative; grid-template-columns: 2 3 4; \
             grid-template-rows: 1 2 }} .abs {{ position: absolute; inset: 0; {placement} }}"
        ),
        20,
        6,
    );
    let r = rect(&dom, abs);
    (r.x, r.y, r.width, r.height)
}

/// §9.1: "the containing block corresponds to the grid area determined
/// by its grid-placement properties".
#[test]
fn an_absolutely_positioned_box_fills_its_grid_area() {
    assert_eq!(placed("grid-column: 2 / 4; grid-row: 2"), (2, 1, 7, 2));
}

/// §9.1: an `auto` line is the containing block's edge — here the rows'
/// both edges, so the area is the whole grid container's height; and
/// `grid-column: 2` ends at the container's right edge, its end `auto`
/// (no span of one: absolutely positioned boxes are not auto-placed).
#[test]
fn an_auto_line_is_the_containing_blocks_edge() {
    assert_eq!(placed("grid-column: 2 / 3"), (2, 0, 3, 3));
    assert_eq!(placed("grid-column: 2"), (2, 0, 18, 3));
    assert_eq!(
        placed("grid-column: span 2; grid-row: 2 / span 1"),
        (0, 1, 20, 2)
    );
}

/// §9.1: a line past the grid — "a non-existent line" — is treated as
/// `auto`.
#[test]
fn a_line_past_the_grid_is_auto() {
    assert_eq!(
        placed("grid-column: 9 / 10; grid-row: 1 / 2"),
        (0, 0, 20, 1)
    );
}

/// §9.1 with §7.1: under `direction: rtl` the column lines run from the
/// right edge, so the first column's area is the rightmost cells.
#[test]
fn an_rtl_grids_areas_run_from_the_right() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let abs = el(&mut dom, g, "div", "abs");
    lay_out(
        &mut dom,
        ".g { display: grid; position: relative; direction: rtl; grid-template-columns: 2 3; \
         grid-template-rows: 1 } .abs { position: absolute; inset: 0; grid-column: 1 / 2; \
         grid-row: 1 / 2 }",
        20,
        4,
    );
    let r = rect(&dom, abs);
    assert_eq!((r.x, r.width), (18, 2));
}

/// The rect of `.abs` (and of `.cb`'s `::before`, if `css` gives it one)
/// in `.cb`, laid out with `css` at 20 × 8.
fn in_cb(css: &str) -> (i32, i32, u16, u16) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let abs = el(&mut dom, cb, "div", "abs");
    lay_out(&mut dom, css, 20, 8);
    let r = rect(&dom, abs);
    (r.x, r.y, r.width, r.height)
}

/// CSS 2.1 §10.1 (and CSS Position 3 §2.1): "the containing block is
/// formed by the padding edge of the ancestor" — an absolutely
/// positioned box's insets count from inside its positioned ancestor's
/// border, not from its border box (C7-ABSPOS-PADDING-EDGE).
#[test]
fn an_absolutely_positioned_box_is_placed_in_the_padding_box() {
    let css = ".cb { position: relative; border: solid; padding: 1; width: 10; height: 3 } \
               .abs { position: absolute; top: 0; left: 0; width: 2; height: 1 }";
    assert_eq!(in_cb(css), (1, 1, 2, 1));
    let css = ".cb { position: relative; border: solid; padding: 1; width: 10; height: 3 } \
               .abs { position: absolute; inset: 0 }";
    assert_eq!(in_cb(css), (1, 1, 12, 5));
}

/// CSS Grid 2 §9.1 with CSS 2.1 §10.1: an `auto` grid line of an
/// absolutely positioned box is the containing block's padding edge.
#[test]
fn an_auto_grid_line_is_the_padding_edge() {
    let css = ".cb { display: grid; position: relative; border: solid; \
               grid-template-columns: 2 3; grid-template-rows: 2 } \
               .abs { position: absolute; inset: 0; grid-column: 2 }";
    assert_eq!(in_cb(css), (3, 1, 16, 2));
}
