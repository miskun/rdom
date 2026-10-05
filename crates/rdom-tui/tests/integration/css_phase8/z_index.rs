//! C8-Z-INDEX — `z-index: <integer>` over the whole integer range (CSS
//! 2.1 §9.9.1): stacking levels compare as integers, ties in tree order
//! (Appendix E), whatever their size.

use super::{el, rows};
use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect};

/// Two overlapping one-cell boxes, `.a` (glyph `a`) then `.b` (`b`) in
/// tree order, with `z-index` `za` and `zb`: the glyph painted on top.
fn top(za: &str, zb: &str) -> String {
    let mut dom = TuiDom::new();
    let root = dom.root();
    for class in ["a", "b"] {
        let e = el(&mut dom, root, "div", class);
        let t = dom.create_text_node(class);
        dom.append_child(e, t).unwrap();
    }
    let css = format!(
        ".a, .b {{ position: absolute; top: 0; left: 0; width: 1; height: 1 }} \
         .a {{ z-index: {za} }} .b {{ z-index: {zb} }}"
    );
    let sheet = rdom_css::from_css_strict(&css).expect("the sheet parses");
    dom.cascade(&sheet);
    let area = Rect::new(0, 0, 2, 1);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    rows(&buf, 1, 1).remove(0)
}

/// §9.9.1: "the higher the stacking level, the more in front" — past the
/// 16-bit range as within it.
#[test]
fn a_higher_level_paints_on_top_across_the_integer_range() {
    assert_eq!(top("40000", "39999"), "a");
    assert_eq!(top("-40000", "-39999"), "b");
    assert_eq!(top("2147483647", "-2147483648"), "a");
    assert_eq!(top("40000", "40000"), "b", "a tie keeps tree order");
}
