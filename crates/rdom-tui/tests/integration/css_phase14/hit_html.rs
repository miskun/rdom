//! C14-HIT-HTML — decided: a point on the canvas outside every box hits
//! nothing (DIVERGENCES §2, `Dom::root()` entry). CSSOM View §5's
//! `elementFromPoint` returns `<html>` there, an ancestor of every box;
//! rdom's document element is the root fragment's first element child,
//! which need not hold the others, so hitting it from the canvas would send
//! canvas clicks, hovers and light-dismiss decisions to an arbitrary
//! top-level element.

use rdom_tui::HitTestExt;

use super::*;

/// The canvas below the content hits nothing — with an `<html>` document
/// element too — while the content hits as before.
#[test]
fn the_canvas_hits_nothing() {
    let mut dom = doc(r#"<html id="h"><body id="b"><p id="p">x</p></body></html>"#);
    styled(&mut dom, "body, p { margin: 0 }", 10, 5);
    assert_eq!(dom.hit_test(0, 0), Some(by_id(&dom, "p")));
    assert_eq!(dom.hit_test(5, 4), None);
    assert!(dom.hit_test_path(5, 4).is_empty());
}
