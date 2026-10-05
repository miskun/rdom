//! C5G-SIZING-SITES — every intrinsic contribution goes through the
//! box's `box-sizing` and its `min-*` / `max-*` (CSS Sizing 3 §5.2: a
//! box's min- / max-content contribution is its outer size with its
//! preferred size, if definite, in place of the content, "and with its
//! min and max sizes applied"; CSS UI 3 §3.1 for which box they measure).

use super::{el, lay_out, size};
use rdom_tui::{NodeId, TuiDom};

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

/// `<p>Save <button style="min-width: 10">OK</button></p>`: the button
/// is an inline block (shrink-to-fit, CSS 2.1 §10.3.9) whose content is
/// `[ OK ]` (6 cells); its `min-width` floors it at 10 (the UA makes a
/// button `border-box`, so 10 is its border box).
#[test]
fn an_inline_blocks_min_width_floors_its_width() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text(&mut dom, p, "Save ");
    let b = el(&mut dom, p, "button", "b");
    text(&mut dom, b, "OK");
    lay_out(&mut dom, ".b { min-width: 10 }", 30, 3);
    assert_eq!(size(&dom, b).0, 10);
}

/// The same atom in an inline formatting context of its own (an inline
/// sibling makes the `<p>` one) and capped by `max-width`.
#[test]
fn an_inline_blocks_max_width_caps_its_width_in_a_line() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let i = el(&mut dom, p, "i", "");
    text(&mut dom, i, "Save ");
    let s = el(&mut dom, p, "span", "s");
    text(&mut dom, s, "abcdef");
    lay_out(
        &mut dom,
        ".s { display: inline-block; max-width: 3 }",
        30,
        3,
    );
    assert_eq!(size(&dom, s).0, 3);
}

/// A percentage width resolves against the containing block when it is
/// definite — the line's block container, 20 wide — instead of being
/// measured from the content; `content-box` adds the padding.
#[test]
fn an_inline_blocks_percentage_width_resolves() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "p");
    let i = el(&mut dom, p, "i", "");
    text(&mut dom, i, "a");
    let s = el(&mut dom, p, "span", "s");
    text(&mut dom, s, "b");
    lay_out(
        &mut dom,
        ".p { width: 20 } .s { display: inline-block; width: 50%; padding: 0 1 }",
        30,
        3,
    );
    assert_eq!(size(&dom, s).0, 12);
    lay_out(
        &mut dom,
        ".p { width: 20 } .s { display: inline-block; width: calc(25% + 1); padding: 0 1 }",
        30,
        3,
    );
    assert_eq!(size(&dom, s).0, 8);
}

/// A flex item's main-size contribution to a shrink-to-fit container:
/// an `auto`-width column flex container in a row is as wide as its
/// widest item's contribution — with that item's `min-width` applied.
#[test]
fn a_flex_items_min_width_reaches_its_containers_contribution() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let col = el(&mut dom, row, "div", "col");
    let item = el(&mut dom, col, "div", "item");
    text(&mut dom, item, "ab");
    lay_out(
        &mut dom,
        ".row { display: flex; flex-direction: row } \
         .col { display: flex; flex-direction: column } \
         .item { min-width: 7 }",
        30,
        3,
    );
    assert_eq!(size(&dom, col).0, 7);
}
