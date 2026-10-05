//! C7G-STACKING-ONE — one answer to "does this box establish a stacking
//! context?" (CSS 2.1 Appendix E, CSS Flexbox §5.4, CSS Grid 2 §6.5):
//! every box painted from its stacking context's layers either is
//! positioned or establishes a context of its own, so the paint and hit
//! walks need no patch of their own.

use super::*;
use crate::{CascadeExt, TuiDom};

/// The box tree `.c > .i`, cascaded with `css`; `(c, i)`.
fn tree(css: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("div");
    dom.set_attribute(c, "class", "c").unwrap();
    dom.append_child(root, c).unwrap();
    let i = dom.create_element("div");
    dom.set_attribute(i, "class", "i").unwrap();
    dom.append_child(c, i).unwrap();
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    (dom, c, i)
}

fn answers(css: &str) -> (bool, bool) {
    let (dom, c, i) = tree(css);
    let style = dom.node(i).ext().unwrap().computed.clone().unwrap();
    (
        is_layered(&dom, i, c, &style),
        creates_stacking_context(&dom, c, &style),
    )
}

/// CSS Flexbox §5.4 / CSS Grid 2 §6.5: a flex or grid item whose
/// `z-index` is not `auto` "create[s] a stacking context even if
/// `position` is `static`" — layered, and a context.
#[test]
fn a_z_indexed_item_is_layered_and_a_stacking_context() {
    for flow in ["flex", "grid"] {
        let css = format!(".c {{ display: {flow} }} .i {{ z-index: 1 }}");
        assert_eq!(answers(&css), (true, true), "{flow}");
    }
}

/// The rule is the item's: a `z-index` on a static block box in block
/// flow does nothing (CSS 2.1 §9.9.1), and a positioned box with
/// `z-index: auto` is layered but no context.
#[test]
fn the_rule_is_the_items() {
    assert_eq!(answers(".i { z-index: 1 }"), (false, false));
    assert_eq!(answers(".i { position: relative }"), (true, false));
    assert_eq!(
        answers(".i { position: relative; z-index: 0 }"),
        (true, true)
    );
    assert_eq!(answers(".i { opacity: 0.5 }"), (false, true));
}
