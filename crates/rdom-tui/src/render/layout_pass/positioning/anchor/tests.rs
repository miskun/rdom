//! Anchor positioning's cost (C15-ANCHOR): a page without an anchored box
//! builds no anchor index; one with anchored boxes builds it once per
//! placement pass, however many ask.

use super::INDEX_BUILDS;
use crate::prelude::*;

/// `n` absolutely positioned boxes, anchored to `#a` when `anchored`, laid
/// out: the anchor indexes built.
fn index_builds(n: usize, anchored: bool) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    dom.set_attribute(a, "id", "a").unwrap();
    dom.append_child(root, a).unwrap();
    for _ in 0..n {
        let p = dom.create_element("div");
        dom.set_attribute(p, "class", "p").unwrap();
        dom.append_child(root, p).unwrap();
    }
    let anchor = if anchored {
        "top: anchor(--a bottom)"
    } else {
        "top: 1"
    };
    let sheet = rdom_css::from_css_strict(&format!(
        "#a {{ anchor-name: --a }} .p {{ position: absolute; {anchor} }}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    INDEX_BUILDS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 40, 10));
    INDEX_BUILDS.with(|c| c.get())
}

#[test]
fn only_anchored_boxes_build_the_anchor_index() {
    assert_eq!(index_builds(8, false), 0);
    let rounds = crate::render::layout_pass::MAX_ROUNDS;
    let built = index_builds(8, true);
    assert!(
        (1..=rounds).contains(&built),
        "{built} index builds for 8 anchored boxes"
    );
}
