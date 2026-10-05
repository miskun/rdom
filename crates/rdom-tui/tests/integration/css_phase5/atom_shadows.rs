//! C5G-FLEX-SHADOW / C5G-INLINE-BLOCK-SHADOW — atomic boxes paint their
//! shadows at their own turn. Flex items paint exactly as inline blocks
//! (CSS Flexbox §5.4), and an inline block paints atomically, as if it
//! created a stacking context (CSS 2.1 Appendix E, step 7.2.1.4.1.1):
//! its own `box-shadow` (with its background, Backgrounds 3 §7.2) and
//! its in-flow blocks' shadows lie over everything painted before it —
//! an earlier item's text included.

use super::{el, paint, rows};
use rdom_tui::{Color, NodeId, TuiDom};

const RED: Color = Color::Rgb(255, 0, 0);

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

/// A flex item's shadow spilling over the item before it covers that
/// item's text: the second item paints whole after the first.
#[test]
fn a_flex_items_shadow_covers_an_earlier_items_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "");
    text(&mut dom, a, "aaaa");
    let b = el(&mut dom, f, "div", "b");
    text(&mut dom, b, "b");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-direction: row } .b { width: 2; box-shadow: -2 0 0 0 red }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), vec!["aa  b "]);
    assert_eq!(buf.cell(2, 0).unwrap().bg, RED);
    assert_eq!(buf.cell(3, 0).unwrap().bg, RED);
    assert_ne!(buf.cell(1, 0).unwrap().bg, RED);
}

/// A block inside a flex item is in the item's paint unit: its shadow
/// paints in the item's background phase, over the earlier item's
/// text, not in the flex container's.
#[test]
fn a_shadow_inside_a_flex_item_covers_an_earlier_items_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "");
    text(&mut dom, a, "aaaa");
    let i = el(&mut dom, f, "div", "");
    let s = el(&mut dom, i, "div", "s");
    text(&mut dom, s, "b");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-direction: row } .s { width: 2; box-shadow: -2 0 0 0 red }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), vec!["aa  b "]);
    assert_eq!(buf.cell(2, 0).unwrap().bg, RED);
    assert_eq!(buf.cell(3, 0).unwrap().bg, RED);
}

/// Inside one flex item the Appendix E order still holds: a later
/// block's shadow lies under an earlier block's text of the same item.
#[test]
fn inside_a_flex_item_a_shadow_stays_under_earlier_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = el(&mut dom, f, "div", "i");
    let a = el(&mut dom, i, "div", "a");
    text(&mut dom, a, "aaaa");
    let b = el(&mut dom, i, "div", "b");
    text(&mut dom, b, "bbbb");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-direction: row } .i { width: 6 } .a, .b { margin-left: 1; width: 4 } \
         .b { box-shadow: 0 -1 0 0 red }",
        6,
        2,
    );
    assert_eq!(rows(&buf, 6, 2), vec![" aaaa ", " bbbb "]);
    for x in 1..5 {
        assert_eq!(buf.cell(x, 0).unwrap().bg, RED, "({x}, 0)");
    }
}
