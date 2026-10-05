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

/// C5G-INLINE-BLOCK-SHADOW: an inline block in a line (an inline
/// formatting context: the `<i>` makes `<p>` one) paints its
/// `box-shadow` at its turn (CSS 2.1 Appendix E, 7.2.1.4.1.1 — atomic,
/// as if it created a stacking context), so the shade covers the text
/// painted before it in the line.
#[test]
fn an_inline_blocks_shadow_paints_at_its_turn() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let i = el(&mut dom, p, "i", "");
    text(&mut dom, i, "aaaa");
    let ib = el(&mut dom, p, "span", "ib");
    text(&mut dom, ib, "b");
    let buf = paint(
        &mut dom,
        ".ib { display: inline-block; width: 2; box-shadow: -2 0 0 0 red }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), vec!["aa  b "]);
    assert_eq!(buf.cell(2, 0).unwrap().bg, RED);
    assert_eq!(buf.cell(3, 0).unwrap().bg, RED);
    assert_ne!(buf.cell(4, 0).unwrap().bg, RED, "not under its own box");
}

/// A translucent inline-block shadow composites over the line's text
/// cells, keeping the glyphs (CSS Color 4 §4.2).
#[test]
fn a_translucent_inline_block_shadow_keeps_the_glyphs_beneath() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let i = el(&mut dom, p, "i", "");
    text(&mut dom, i, "aaaa");
    let ib = el(&mut dom, p, "span", "ib");
    text(&mut dom, ib, "b");
    let buf = paint(
        &mut dom,
        ".ib { display: inline-block; width: 2; box-shadow: -2 0 rgb(255 0 0 / 50%) }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), vec!["aaaab "]);
    assert!(
        matches!(buf.cell(2, 0).unwrap().bg, Color::Rgb(r, 0, 0) if r > 100),
        "{:?}",
        buf.cell(2, 0).unwrap().bg
    );
}

/// An inline block beside bare text (no inline element: the block lays
/// it out through an anonymous box) composites a translucent shadow
/// once, as in a line box — the same shade as the IFC case.
#[test]
fn an_inline_blocks_translucent_shadow_composites_once_beside_bare_text() {
    let shade = |wrap_text: bool| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "p", "");
        if wrap_text {
            let i = el(&mut dom, p, "i", "");
            text(&mut dom, i, "aaaa");
        } else {
            text(&mut dom, p, "aaaa");
        }
        let ib = el(&mut dom, p, "span", "ib");
        text(&mut dom, ib, "b");
        let buf = paint(
            &mut dom,
            "p { background-color: rgb(0 0 255) } \
             .ib { display: inline-block; width: 2; box-shadow: -2 0 rgb(255 0 0 / 50%) }",
            6,
            1,
        );
        buf.cell(2, 0).unwrap().bg
    };
    assert_eq!(shade(false), shade(true));
}
