//! C6-VISIBILITY — `visibility: visible | hidden | collapse` (CSS
//! Display 3 §4) in layout, paint, hit testing and focus.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, Color, HitTestExt, LayoutExt, PaintExt, TuiDom, TuiNodeExt};

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, t: &str) {
    let n = dom.create_text_node(t);
    dom.append_child(parent, n).unwrap();
}

/// CSS Display 3 §4: a `hidden` box keeps its place and size but draws
/// nothing — no background, border or text — while a descendant set
/// back to `visible` is drawn. The property inherits.
#[test]
fn hidden_keeps_its_space_and_a_visible_child_shows() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    text(&mut dom, a, "aa");
    let v = el(&mut dom, a, "span", "v");
    text(&mut dom, v, "vv");
    let i = el(&mut dom, a, "span", "");
    text(&mut dom, i, "ii");
    let b = el(&mut dom, root, "div", "");
    text(&mut dom, b, "bb");
    let buf = paint(
        &mut dom,
        ".a { visibility: hidden; border: solid; background-color: red; width: 10 } \
         .v { visibility: visible }",
        12,
        5,
    );
    assert_eq!(
        rows(&buf, 10, 4),
        ["          ", "   vv     ", "          ", "bb        "]
    );
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Reset);
    assert_eq!(rect(&dom, b).y, 3);
    let hidden = rdom_tui::Visibility::Hidden;
    assert_eq!(dom.node(i).computed().unwrap().visibility, hidden);
}

/// CSS Display 3 §4 / UI 4: a hidden box is not a hit target — the
/// point falls through to what is beneath — and its visible
/// descendant is, with the hidden ancestor on its path.
#[test]
fn hidden_boxes_are_not_hit_but_visible_descendants_are() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let a = el(&mut dom, p, "div", "a");
    text(&mut dom, a, "aa ");
    let v = el(&mut dom, a, "span", "v");
    text(&mut dom, v, "vv");
    lay_out(
        &mut dom,
        ".p { width: 10 } .a { visibility: hidden; width: 8 } .v { visibility: visible }",
        12,
        3,
    );
    assert_eq!(dom.hit_test(0, 0), Some(p));
    assert_eq!(dom.hit_test(3, 0), Some(v));
    assert!(dom.hit_test_path(3, 0).contains(&a));
}

/// HTML §6.6.3: an element that is not being rendered visibly is not
/// focusable — a `visibility: hidden` button is skipped by Tab, its
/// `visible` descendant is not.
#[test]
fn hidden_elements_are_not_focusable() {
    use rdom_tui::runtime::focus::tabindex::focusable_elements;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = el(&mut dom, root, "div", "h");
    let hb = el(&mut dom, h, "button", "");
    text(&mut dom, hb, "x");
    let vb = el(&mut dom, h, "button", "v");
    text(&mut dom, vb, "y");
    let c = el(&mut dom, root, "button", "c");
    text(&mut dom, c, "z");
    lay_out(
        &mut dom,
        ".h { visibility: hidden } .v { visibility: visible } .c { visibility: collapse }",
        12,
        5,
    );
    assert_eq!(focusable_elements(&dom), [vb]);
}

/// CSS Flexbox §4.4: a collapsed flex item is removed from rendering
/// but leaves a strut — zero main size, its cross size kept for the
/// line — so its siblings close up and the container keeps its height.
#[test]
fn a_collapsed_flex_item_leaves_a_strut() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "i");
    text(&mut dom, a, "a");
    let b = el(&mut dom, f, "div", "i b");
    text(&mut dom, b, "b");
    let c = el(&mut dom, f, "div", "i");
    text(&mut dom, c, "c");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-direction: row } .i { width: 3 } \
         .b { visibility: collapse; height: 3; margin: 0 2 }",
        12,
        4,
    );
    assert_eq!(rect(&dom, c).x, 3);
    assert_eq!(rect(&dom, b).width, 0);
    assert_eq!(rect(&dom, f).height, 3);
    assert_eq!(rows(&buf, 7, 1), ["a  c   "]);
}

/// CSS Display 3 §4: outside flex items and table rows, `collapse` is
/// `hidden` — the box keeps its space.
#[test]
fn collapse_elsewhere_is_hidden() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    text(&mut dom, a, "aa");
    let b = el(&mut dom, root, "div", "");
    text(&mut dom, b, "bb");
    let buf = paint(&mut dom, ".a { visibility: collapse; height: 2 }", 6, 4);
    assert_eq!(rect(&dom, b).y, 2);
    assert_eq!(rows(&buf, 2, 3), ["  ", "  ", "bb"]);
}

/// CSS 2.1 §17.5.5: a collapsed table row takes no space, and the
/// column widths are what they are with it (its cells still size the
/// columns).
#[test]
fn a_collapsed_table_row_takes_no_space_but_sizes_its_columns() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", "");
    let r1 = el(&mut dom, t, "tr", "c");
    let wide = el(&mut dom, r1, "td", "");
    text(&mut dom, wide, "wwwwww");
    let r2 = el(&mut dom, t, "tr", "");
    let cell = el(&mut dom, r2, "td", "");
    text(&mut dom, cell, "x");
    let sheet = rdom_css::from_css_strict(".c { visibility: collapse }").unwrap();
    dom.cascade(&sheet);
    rdom_tui::runtime::builtins::table::size_all_tables(&mut dom);
    dom.layout_dom(Rect::new(0, 0, 20, 4));
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 20, 4));
    assert_eq!(rect(&dom, r2).y, 0);
    assert_eq!(rect(&dom, cell).width, 6 + 2);
    assert_eq!(rows(&buf, 8, 1), [" x      "]);
}
