//! C15-FILTER — what the graphical effects cost a page that has none: no
//! layer, no coverage, no allocation; and one layer per effect group.

use super::effects::EFFECT_LAYERS;
use crate::prelude::*;

/// Paint `<body>` holding `n` styled paragraphs under `css`: the effect
/// layers made and the allocations of the paint.
fn paint_counts(n: usize, css: &str) -> (usize, u64) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body).unwrap();
    for _ in 0..n {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(body, p).unwrap();
    }
    let sheet = rdom_css::from_css_strict(css).unwrap();
    let area = Rect::new(0, 0, 20, 60);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    assert!(!buf.tracks_coverage(), "the frame tracks no coverage");
    EFFECT_LAYERS.with(|c| c.set(0));
    let allocations = crate::test_alloc::allocations_in(|| dom.paint_dom(&mut buf, area));
    (EFFECT_LAYERS.with(|c| c.get()), allocations)
}

/// A page without filters pays nothing for them: no layer, and declaring
/// `filter: none` / `backdrop-filter: none` on every element allocates
/// nothing more than not declaring them. A filtered element costs one
/// layer, the cells of its group.
#[test]
fn a_page_without_effects_makes_no_layer() {
    let plain = "p { background-color: rgb(0, 0, 200); margin: 0 }";
    let (layers, base) = paint_counts(30, plain);
    assert_eq!(layers, 0);
    let (layers, none) = paint_counts(
        30,
        &format!("{plain} p {{ filter: none; backdrop-filter: none }}"),
    );
    assert_eq!((layers, none), (0, base));
    let (layers, _) = paint_counts(
        30,
        &format!("{plain} p:first-child {{ filter: invert(1) }}"),
    );
    assert_eq!(layers, 1);
    // `blur()` and `url()` draw nothing: still no layer.
    let (layers, _) = paint_counts(30, &format!("{plain} p {{ filter: blur(2px) url(#x) }}"));
    assert_eq!(layers, 0);
}
