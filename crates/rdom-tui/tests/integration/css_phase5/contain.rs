//! C5-CONTAIN-SIZE — `contain-intrinsic-size` and its longhands (CSS
//! Sizing 4 §6.1) cascade to the computed style. They size a box only
//! under size containment, which arrives with C14 `contain`.

use super::{el, lay_out};
use rdom_tui::layout::ContainIntrinsicSize;
use rdom_tui::{TuiDom, TuiNodeExt};

fn computed(css: &str) -> (ContainIntrinsicSize, ContainIntrinsicSize) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    lay_out(&mut dom, css, 40, 10);
    let c = dom.node(b).computed().expect("cascaded").clone();
    (c.contain_intrinsic_width, c.contain_intrinsic_height)
}

/// CSS Sizing 4 §6.1: initial `none`, not inherited; a declared value
/// reaches the computed style, its viewport units absolute (CSS Values
/// 4 §6.1.2: 25vw of a 40-column terminal is 10 cells).
#[test]
fn contain_intrinsic_size_cascades() {
    let (w, h) = computed("");
    assert_eq!((w.auto, w.cells(), h.cells()), (false, None, None));
    let (w, h) = computed(".b { contain-intrinsic-size: auto 25vw 3 }");
    assert_eq!(
        (w.auto, w.cells(), h.auto, h.cells()),
        (true, Some(10), false, Some(3))
    );
    let (w, _) =
        computed("div { contain-intrinsic-width: 4 } .b { contain-intrinsic-width: inherit }");
    assert_eq!(w.cells(), None, "inherits the root's initial value");
}

/// CSS Logical 1 §4: the logical longhand and its physical twin are one
/// property pair — whichever is declared later (or wins the cascade)
/// applies.
#[test]
fn logical_and_physical_longhands_cascade_in_order() {
    let (w, _) = computed(".b { contain-intrinsic-inline-size: 5; contain-intrinsic-width: 6 }");
    assert_eq!(w.cells(), Some(6));
    let (w, _) = computed(".b { contain-intrinsic-width: 6; contain-intrinsic-inline-size: 5 }");
    assert_eq!(w.cells(), Some(5));
    let (_, h) =
        computed("div.b { contain-intrinsic-height: 1 } .b { contain-intrinsic-block-size: 2 }");
    assert_eq!(h.cells(), Some(1), "the more specific rule wins");
}
