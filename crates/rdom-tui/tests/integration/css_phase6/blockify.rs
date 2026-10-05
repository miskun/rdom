//! C6G-BLOCKIFY — a flex item's computed `display` is blockified (CSS
//! Display 3 §2.7, CSS Flexbox §4), at computed-value time, so every
//! reader of the computed style and every layout path agree.

use super::{el, lay_out, size};
use rdom_tui::layout::{Display, Flow};
use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

fn display(dom: &TuiDom, id: NodeId) -> (Display, Flow) {
    let c = dom.node(id).computed().expect("computed");
    (c.display, c.flow)
}

/// CSS Display 3 §2.7: "The computed display value of a flex item is
/// blockified": `inline` → `block`, `inline-block` (`inline flow-root`)
/// → `block flow-root`, `inline-flex` → `flex`, `inline flow-root` →
/// `flow-root`; `block`, `contents` and `none` are unaffected.
#[test]
fn a_flex_items_display_is_blockified() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "span", "");
    let b = el(&mut dom, f, "span", "ib");
    let c = el(&mut dom, f, "span", "if");
    let d = el(&mut dom, f, "span", "ifr");
    let e = el(&mut dom, f, "span", "n");
    lay_out(
        &mut dom,
        ".f { display: flex } .ib { display: inline-block } .if { display: inline-flex } \
         .ifr { display: inline flow-root } .n { display: none }",
        20,
        2,
    );
    assert_eq!(display(&dom, a), (Display::Block, Flow::Block));
    assert_eq!(display(&dom, b), (Display::Block, Flow::FlowRoot));
    assert_eq!(display(&dom, c), (Display::Block, Flow::Flex));
    assert_eq!(display(&dom, d), (Display::Block, Flow::FlowRoot));
    assert_eq!(display(&dom, e).0, Display::None);
}

/// CSS Display 3 §2.5: a `contents` element generates no box, so its
/// children's parent box is the flex container — they are flex items
/// and blockified; the `contents` element itself is not.
#[test]
fn the_children_of_a_contents_child_of_a_flex_container_are_blockified() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let c = el(&mut dom, f, "span", "c");
    let s = el(&mut dom, c, "span", "");
    lay_out(
        &mut dom,
        ".f { display: flex } .c { display: contents }",
        20,
        2,
    );
    assert_eq!(display(&dom, c).0, Display::Contents);
    assert_eq!(display(&dom, s), (Display::Block, Flow::Block));
}

/// CSS Flexbox §4: a flex container's `::before` / `::after` are child
/// boxes, so flex items, and blockified.
#[test]
fn a_flex_containers_pseudo_elements_are_blockified() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    lay_out(
        &mut dom,
        ".f { display: flex } .f::before { content: \"x\"; display: inline-block }",
        20,
        2,
    );
    let before = dom.node(f).computed_before().expect("::before");
    assert_eq!(
        (before.display, before.flow),
        (Display::Block, Flow::FlowRoot)
    );
}

/// The document root's children are not flex items: rdom lays them out
/// in its viewport column only as a layout device standing in for a
/// browser's `<body>`, whose children are blocks and inlines in normal
/// flow. So they keep their `display`, and an inline block among them
/// sits at its content width.
#[test]
fn the_document_roots_children_are_not_blockified() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "span", "ib");
    let t = dom.create_text_node("ab");
    dom.append_child(a, t).unwrap();
    lay_out(&mut dom, ".ib { display: inline-block }", 20, 2);
    assert_eq!(display(&dom, a).0, Display::InlineBlock);
    assert_eq!(size(&dom, a).0, 2);
}

/// A flex container that stops being one un-blockifies its items.
#[test]
fn an_item_of_a_former_flex_container_is_inline_again() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let s = el(&mut dom, f, "span", "");
    lay_out(&mut dom, ".f { display: flex }", 20, 2);
    assert_eq!(display(&dom, s).0, Display::Block);
    dom.set_attribute(f, "class", "g").unwrap();
    lay_out(&mut dom, ".f { display: flex }", 20, 2);
    assert_eq!(display(&dom, s).0, Display::Inline);
}
