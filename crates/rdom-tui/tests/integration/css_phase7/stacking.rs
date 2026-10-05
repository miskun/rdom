//! C7-GRID-PLACE — grid and flex items in the stacking order (CSS Grid 2
//! §6.5, CSS Flexbox §5.4): "`z-index` values other than `auto` create a
//! stacking context even if `position` is `static`".

use super::{el, paint, rows};
use rdom_tui::{HitTestExt, TuiDom};

/// Two overlapping items in `container_css` — `a` pulled one cell over
/// `b` by a negative margin — with `item_css`; the first four cells and
/// the item hit at the overlap.
fn overlap(container_css: &str, item_css: &str) -> (String, bool) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = el(&mut dom, g, "span", "a");
    let t = dom.create_text_node("aa");
    dom.append_child(a, t).unwrap();
    let b = el(&mut dom, g, "span", "b");
    let t = dom.create_text_node("bb");
    dom.append_child(b, t).unwrap();
    let buf = paint(
        &mut dom,
        &format!("{container_css} .b {{ margin-left: -1 }} {item_css}"),
        4,
        1,
    );
    let hit_a = dom.hit_test(1, 0) == Some(a);
    (rows(&buf, 4, 1).remove(0), hit_a)
}

const GRID: &str = ".g { display: grid; grid-template-columns: 2 2 }";
const FLEX: &str = ".g { display: flex }";

/// Without `z-index` the later item paints over the earlier one; with a
/// positive `z-index` the earlier one — a static grid item — paints on
/// top and is hit first.
#[test]
fn z_index_lifts_a_static_grid_item() {
    assert_eq!(overlap(GRID, ""), ("abb ".to_string(), false));
    assert_eq!(
        overlap(GRID, ".a { z-index: 1 }"),
        ("aab ".to_string(), true)
    );
}

/// CSS Flexbox §5.4: the same for a static flex item.
#[test]
fn z_index_lifts_a_static_flex_item() {
    assert_eq!(overlap(FLEX, ""), ("abb ".to_string(), false));
    assert_eq!(
        overlap(FLEX, ".a { z-index: 1 }"),
        ("aab ".to_string(), true)
    );
}

/// CSS 2.1 Appendix E: a negative `z-index` stacking context paints
/// before its stacking context's in-flow boxes — so a static grid item
/// with `z-index: -1` lies under its container's background, and a point
/// on it hits the container.
#[test]
fn a_negative_z_index_item_paints_under_its_container() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = el(&mut dom, g, "span", "a");
    let t = dom.create_text_node("x");
    dom.append_child(a, t).unwrap();
    let buf = paint(
        &mut dom,
        ".g { display: grid; background-color: blue } .a { z-index: -1 }",
        3,
        1,
    );
    assert_eq!(rows(&buf, 3, 1), ["   "]);
    assert_eq!(dom.hit_test(0, 0), Some(g));
}
