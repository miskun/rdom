//! C6G-PSEUDO-FLEX-ITEMS — a `::before` / `::after` flex item is a box
//! of its own (CSS Flexbox §4: the pseudo-elements of a flex container
//! are child boxes, so flex items): its sizes, `flex`, `order`, margins,
//! padding, border and alignment apply, and its box paints.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::{Color, TuiDom};

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, data: &str) {
    let t = dom.create_text_node(data);
    dom.append_child(parent, t).unwrap();
}

/// CSS Flexbox §4 with CSS Box 4 / CSS UI 3: the item's border box is
/// its margin-start offset, then its `width` (a content box,
/// `box-sizing: content-box`) plus its padding and border; its text
/// sits inside them.
#[test]
fn a_pseudo_element_item_has_its_own_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let b = el(&mut dom, f, "div", "b");
    text(&mut dom, b, "x");
    let buf = paint(
        &mut dom,
        ".f { display: flex; width: 20 } \
         .f::before { content: 'ab'; width: 3; margin-left: 2; padding-left: 1; border: solid }",
        20,
        3,
    );
    // Border box: x 2, 1 + 1 + 3 + 1 = 6 wide, 3 rows tall.
    assert_eq!(rect(&dom, b).x, 8);
    assert_eq!(rect(&dom, f).height, 3);
    assert_eq!(rows(&buf, 9, 3), ["  ┌────┐x", "  │ ab │ ", "  └────┘ "]);
}

/// CSS Flexbox §7 and §5.4: the item's `flex` grows it into the free
/// space and its `order` places it.
#[test]
fn a_pseudo_element_item_flexes_and_is_ordered() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let b = el(&mut dom, f, "div", "b");
    text(&mut dom, b, "x");
    let buf = paint(
        &mut dom,
        ".f { display: flex; width: 10 } .b { width: 3 } \
         .f::after { content: 'z'; flex: 1; order: -1; background-color: red }",
        10,
        1,
    );
    assert_eq!(rect(&dom, b).x, 7);
    assert_eq!(rows(&buf, 10, 1), ["z      x  "]);
    assert_ne!(
        buf.cell(6, 0).unwrap().bg,
        Color::Reset,
        "the grown box paints"
    );
    assert_eq!(buf.cell(7, 0).unwrap().bg, Color::Reset);
}

/// CSS Box Alignment 3 §6.2 / CSS Flexbox §8.3: the item's own
/// `align-self` places it on the cross axis.
#[test]
fn a_pseudo_element_item_aligns_itself() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "div", "f");
    let buf = paint(
        &mut dom,
        ".f { display: flex; height: 3; align-items: flex-start } \
         .f::before { content: 'p'; align-self: flex-end }",
        4,
        3,
    );
    assert_eq!(rows(&buf, 4, 3), ["    ", "    ", "p   "]);
}

/// CSS Sizing 3 §5: the container's intrinsic size counts the item's
/// padding and margins.
#[test]
fn a_pseudo_element_items_box_counts_in_the_containers_intrinsic_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    lay_out(
        &mut dom,
        ".f { display: flex; width: max-content } \
         .f::before { content: 'ab'; padding: 0 1; margin-right: 2 }",
        20,
        1,
    );
    assert_eq!(rect(&dom, f).width, 6);
}
