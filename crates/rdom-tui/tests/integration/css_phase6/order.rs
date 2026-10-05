//! C6-ORDER — `order` (CSS Flexbox §5.4): flex items are laid out and
//! painted in order-modified document order; focus and the DOM keep
//! document order.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::{HitTestExt, TuiDom};

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, t: &str) {
    let n = dom.create_text_node(t);
    dom.append_child(parent, n).unwrap();
}

/// CSS Flexbox §5.4: items are placed in ascending `order`, items of
/// equal `order` in document order (`order` 0 is the initial value).
#[test]
fn items_are_placed_in_order_modified_document_order() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "i a");
    let b = el(&mut dom, f, "div", "i b");
    let c = el(&mut dom, f, "div", "i");
    let d = el(&mut dom, f, "div", "i b");
    lay_out(
        &mut dom,
        ".f { display: flex; flex-direction: row } .i { width: 2; height: 1 } \
         .a { order: 2 } .b { order: -1 }",
        12,
        2,
    );
    let xs: Vec<i32> = [a, b, c, d].iter().map(|&n| rect(&dom, n).x).collect();
    assert_eq!(xs, [6, 0, 4, 2]);
}

/// CSS Flexbox §5.4: `order` affects painting order "in the same way"
/// as layout: of two overlapping items the one later in
/// order-modified document order paints on top, and is hit first.
#[test]
fn painting_and_hit_testing_follow_order() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "a");
    text(&mut dom, a, "aa");
    let b = el(&mut dom, f, "div", "b");
    text(&mut dom, b, "bb");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-direction: row } .a { order: 1; margin-left: -1 }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 4, 1), ["baa "]);
    assert_eq!(dom.hit_test(1, 0), Some(a));
    assert_eq!(dom.hit_test(0, 0), Some(b));
}

/// CSS Flexbox §5.4.1: `order` is visual only — sequential focus
/// navigation keeps document order.
#[test]
fn focus_order_keeps_document_order() {
    use rdom_tui::runtime::focus::tabindex::focusable_elements;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let x = el(&mut dom, f, "button", "x");
    text(&mut dom, x, "x");
    let y = el(&mut dom, f, "button", "");
    text(&mut dom, y, "y");
    lay_out(
        &mut dom,
        ".f { display: flex; flex-direction: row } .x { order: 1 }",
        12,
        2,
    );
    assert!(rect(&dom, x).x > rect(&dom, y).x);
    assert_eq!(focusable_elements(&dom), [x, y]);
}
