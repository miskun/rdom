//! C8-OVERFLOW-CLIP — `overflow: clip`, the two-value `overflow`,
//! `overflow-clip-margin` and the computed-value rule (CSS Overflow 3
//! §3): a `clip` axis clips at the overflow clip edge, without making a
//! scroll container or a formatting context.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::layout::Overflow;
use rdom_tui::prelude::*;
use rdom_tui::render::Rect;

/// The computed `(overflow-x, overflow-y)` of a `.p` styled `decl`.
fn computed(decl: &str) -> (Overflow, Overflow) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    lay_out(&mut dom, &format!(".p {{ {decl} }}"), 10, 4);
    let c = dom.node(p).computed().expect("cascaded");
    (c.overflow_x, c.overflow_y)
}

/// §3.1 computed value: "as specified, except with visible/clip
/// computing to auto/hidden (respectively) if one of overflow-x or
/// overflow-y is neither visible nor clip".
#[test]
fn a_visible_or_clip_axis_beside_a_scrollable_one_computes_to_auto_or_hidden() {
    use Overflow::*;
    assert_eq!(
        computed("overflow-x: visible; overflow-y: scroll"),
        (Auto, Scroll)
    );
    assert_eq!(computed("overflow: clip hidden"), (Hidden, Hidden));
    assert_eq!(computed("overflow: auto clip"), (Auto, Hidden));
    // Neither axis scrollable: as specified.
    assert_eq!(computed("overflow: clip visible"), (Clip, Visible));
    assert_eq!(computed("overflow: visible"), (Visible, Visible));
}

/// §3.1: `overflow-block` / `overflow-inline` reach layout as the
/// vertical / horizontal axes (`horizontal-tb`).
#[test]
fn the_logical_longhands_compute_to_the_physical_axes() {
    assert_eq!(
        computed("overflow-block: clip; overflow-inline: visible"),
        (Overflow::Visible, Overflow::Clip)
    );
}

/// `.p` 3 wide, 1 tall, holding two 6-wide rows `abcdef` / `ghijkl`
/// (the second below the box), painted in 8 × 2 with `decl`.
fn clipped(decl: &str) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    for (class, text) in [("r", "abcdef"), ("r", "ghijkl")] {
        let r = el(&mut dom, p, "div", class);
        let t = dom.create_text_node(text);
        dom.append_child(r, t).unwrap();
    }
    let buf = paint(
        &mut dom,
        &format!(".p {{ width: 3; height: 1; {decl} }} .r {{ width: 6; height: 1 }}"),
        8,
        2,
    );
    rows(&buf, 8, 2)
}

/// §3.1: `clip` clips "to the box's overflow clip edge" on its axis
/// alone — with `overflow-y: visible` the second row still paints, cut
/// at the right edge; with both axes `clip` it is gone.
#[test]
fn clip_clips_its_own_axis() {
    assert_eq!(clipped("overflow-x: clip"), ["abc     ", "ghi     "]);
    assert_eq!(clipped("overflow: clip"), ["abc     ", "        "]);
    assert_eq!(clipped("overflow: clip visible"), ["abc     ", "ghi     "]);
}

/// §3.2: "the overflow clip edge ... is the box edge named by the
/// <visual-box> value, outset by the <length>" — on `clip` axes only.
#[test]
fn overflow_clip_margin_moves_the_clip_edge() {
    assert_eq!(
        clipped("overflow-x: clip; overflow-clip-margin: 1"),
        ["abcd    ", "ghij    "]
    );
    assert_eq!(
        clipped("overflow: clip; overflow-clip-margin: 2"),
        ["abcde   ", "ghijk   "]
    );
    // The margin does nothing for a scroll container.
    assert_eq!(
        clipped("overflow: hidden; overflow-clip-margin: 2"),
        ["abc     ", "        "]
    );
    // `content-box`: the edge inside the right padding.
    assert_eq!(
        clipped("padding-right: 1; overflow: clip; overflow-clip-margin: content-box"),
        ["abc     ", "        "]
    );
    assert_eq!(
        clipped("padding-right: 1; overflow: clip"),
        ["abcd    ", "        "]
    );
}

/// §3.1: `clip` "forbids all scrolling, including programmatic
/// scrolling" — the box is not a scroll container: an offset write is
/// refused and a stale offset is dropped at the next layout.
#[test]
fn a_clip_box_cannot_be_scrolled() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let tall = el(&mut dom, p, "div", "tall");
    let css = ".p { height: 3; overflow: clip } .tall { height: 10 }";
    lay_out(&mut dom, css, 10, 6);
    dom.node_mut(p).set_scroll_top(4).unwrap();
    assert_eq!(dom.node(p).scroll_top(), Some(0));
    {
        let mut node = dom.node_mut(p);
        node.ext_mut().unwrap().scroll_y = 4;
    }
    dom.layout_dom(Rect::new(0, 0, 10, 6));
    assert_eq!(dom.node(p).scroll_top(), Some(0));
    assert_eq!(rect(&dom, tall).y, 0);
}

/// §3.1: unlike `hidden`, `clip` "does not cause the element to
/// establish a new formatting context" — its first child's top margin
/// collapses through it (CSS 2.1 §8.3.1).
#[test]
fn a_clip_box_is_no_formatting_context() {
    let top = |overflow: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let w = el(&mut dom, root, "div", "w");
        let p = el(&mut dom, w, "div", "p");
        el(&mut dom, p, "div", "c");
        lay_out(
            &mut dom,
            &format!(".p {{ overflow: {overflow} }} .c {{ margin-top: 2; height: 1 }}"),
            10,
            6,
        );
        rect(&dom, p).y
    };
    assert_eq!(top("clip"), 2, "the margin collapses through");
    assert_eq!(top("hidden"), 0, "a scroll container keeps it inside");
}

/// CSS Overflow 3 §2.2 with §3.1: a `clip` descendant's content counts in
/// a scroll container's scrollable overflow up to its overflow clip edge
/// on its `clip` axis — and wholly on its `visible` one.
#[test]
fn a_clip_descendant_bounds_the_scrollable_overflow_on_its_clip_axis() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let k = el(&mut dom, port, "div", "k");
    el(&mut dom, k, "div", "wide");
    lay_out(
        &mut dom,
        ".port { width: 10; height: 3; overflow: hidden } \
         .k { width: 4; height: 1; overflow-x: clip } .wide { width: 20; height: 5 }",
        12,
        6,
    );
    // The 20-wide row is cut at `.k`'s right edge (4), inside the 10-cell
    // scrollport the area always covers; its 5 rows count.
    let node = dom.node(port);
    assert_eq!(
        (node.scroll_width(), node.scroll_height()),
        (Some(10), Some(5))
    );
}
