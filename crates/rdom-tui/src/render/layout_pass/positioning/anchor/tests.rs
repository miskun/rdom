//! Anchor positioning's cost (C15-ANCHOR): a page without an anchored box
//! builds no anchor index; one with anchored boxes builds it once per
//! placement pass, however many ask.

use super::INDEX_BUILDS;
use super::lookup::ACCEPTABLE_CALLS;
use super::visibility::HIDDEN_WORK;
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

/// `n` rows in a 10-row scroll container, each `anchor-name: --row;
/// anchor-scope: --row` with a tooltip anchored to it — the rows past the
/// fold hide theirs (`anchors-visible`) — laid out and painted: the anchor
/// candidates tested and the hidden-box comparisons made.
fn row_tooltips(n: usize) -> (usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("div");
    dom.set_attribute(list, "class", "list").unwrap();
    dom.append_child(root, list).unwrap();
    for _ in 0..n {
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "row").unwrap();
        let t = dom.create_text_node("row");
        dom.append_child(row, t).unwrap();
        let tip = dom.create_element("span");
        dom.set_attribute(tip, "class", "tip").unwrap();
        let t = dom.create_text_node("tip");
        dom.append_child(tip, t).unwrap();
        dom.append_child(row, tip).unwrap();
        dom.append_child(list, row).unwrap();
    }
    let sheet = rdom_css::from_css_strict(
        ".list { height: 10; overflow: auto } \
         .row { height: 1; anchor-name: --row; anchor-scope: --row } \
         .tip { position: absolute; position-anchor: --row; top: anchor(top); left: anchor(right, 0) }",
    )
    .unwrap();
    dom.cascade(&sheet);
    ACCEPTABLE_CALLS.with(|c| c.set(0));
    HIDDEN_WORK.with(|c| c.set(0));
    let area = Rect::new(0, 0, 40, 12);
    dom.layout_dom(area);
    let mut buf = crate::render::Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    (
        ACCEPTABLE_CALLS.with(|c| c.get()),
        HIDDEN_WORK.with(|c| c.get()),
    )
}

/// Architect N7, N8 (C15G-ANCHOR-COST): n rows each anchoring a tooltip
/// cost linear work — the anchor lookups (memoized per pass, candidates
/// narrowed by `anchor-scope`) and the `position-visibility` checks of
/// paint (no ancestor walk against every hidden box).
#[test]
fn row_tooltips_cost_linear_work() {
    let (a20, h20) = row_tooltips(20);
    let (a80, h80) = row_tooltips(80);
    assert!(a20 > 0, "the lookups ran");
    assert!(
        a80 <= 5 * a20,
        "anchor candidates: {a20} for 20 rows, {a80} for 80"
    );
    assert!(
        h80 <= 5 * h20.max(1),
        "hidden-box work: {h20} for 20 rows, {h80} for 80"
    );
}
