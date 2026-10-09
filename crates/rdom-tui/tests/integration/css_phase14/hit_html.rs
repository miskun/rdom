//! C14-HIT-HTML, implemented by C14G-ROOT-ELEMENT: a point on the canvas
//! outside every box hits the root element — CSSOM View §5's
//! `elementFromPoint` returns `<html>` there, an ancestor of every box,
//! and the root fragment is the root element (DIVERGENCES §2) — so canvas
//! clicks, hovers and light-dismiss decisions go to the ancestor of
//! everything, never to an arbitrary top-level element.

use rdom_tui::HitTestExt;

use super::*;

/// The canvas below the content hits the root — with an `<html>` tree
/// under it too — while the content hits as before.
#[test]
fn the_canvas_hits_the_root() {
    let mut dom = doc(r#"<html id="h"><body id="b"><p id="p">x</p></body></html>"#);
    styled(&mut dom, "body, p { margin: 0 }", 10, 5);
    assert_eq!(dom.hit_test(0, 0), Some(by_id(&dom, "p")));
    assert_eq!(dom.hit_test(5, 4), Some(dom.root()));
    assert_eq!(dom.hit_test_path(5, 4), vec![dom.root()]);
}

/// An element root is the root element too: the canvas below its box
/// hits it.
#[test]
fn the_canvas_hits_an_element_root() {
    let mut dom = TuiDom::with_root_tag("html");
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, "<p>x</p>", root).expect("markup parses");
    styled(&mut dom, "p { margin: 0 }", 10, 5);
    assert_eq!(dom.hit_test(5, 4), Some(root));
}
