//! C8G-IDLE-COST — a paint pays for line markers only where a box asks
//! for them: an inline flow walks up to the line-clamp container it may
//! be in only when the document has one (CSS Overflow 4 §4.3), and a
//! clamp point is read from layout's per-pass cache, not walked per flow
//! per paint.

use super::text_overflow::BLOCK_LINE_WALKS;
use crate::prelude::*;
use crate::render::layout_pass::line_clamp::CLAMP_WALKS;

/// `<body>` holding `n` paragraphs `x` nested `depth` deep, styled by
/// `css`, laid out once and painted `paints` times: the block-line walk
/// steps and the clamp-point walks the paints made.
fn paint_walks(css: &str, n: usize, depth: usize, paints: usize) -> (usize, usize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut parent = dom.create_element("body");
    dom.append_child(root, parent).unwrap();
    for _ in 0..depth {
        let d = dom.create_element("div");
        dom.set_attribute(d, "class", "d").unwrap();
        dom.append_child(parent, d).unwrap();
        parent = d;
    }
    for _ in 0..n {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(parent, p).unwrap();
    }
    let sheet = rdom_css::from_css_strict(css).unwrap();
    let area = Rect::new(0, 0, 20, 100);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    BLOCK_LINE_WALKS.with(|c| c.set(0));
    CLAMP_WALKS.with(|c| c.set(0));
    for _ in 0..paints {
        let mut buf = Buffer::empty(area);
        dom.paint_dom(&mut buf, area);
    }
    (
        BLOCK_LINE_WALKS.with(|c| c.get()),
        CLAMP_WALKS.with(|c| c.get()),
    )
}

/// No line-clamp container anywhere: no inline flow walks to a
/// formatting root, however deep it sits (one walk per flow per level
/// before, every paint).
#[test]
fn without_a_line_clamp_no_flow_walks_up() {
    assert_eq!(paint_walks("p { color: red }", 10, 10, 3), (0, 0));
}

/// A line-clamp container: its clamp point comes from the layout pass,
/// so painting it again and again walks it no more.
#[test]
fn a_clamp_point_is_walked_by_layout_only() {
    let (_, clamp_walks) = paint_walks(".d { line-clamp: 2 }", 5, 1, 3);
    assert_eq!(clamp_walks, 0);
}
