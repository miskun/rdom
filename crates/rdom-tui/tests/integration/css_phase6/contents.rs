//! C6G-CONTENTS-BOXTREE — a `display: contents` element in the box
//! tree (CSS Display 3 §2.5): its children and pseudo-elements are its
//! parent's, wherever the parent walks them — hit-testing under
//! `order`, a flex container's anonymous item, the scrollable overflow,
//! the static position of an out-of-flow child.

use super::{el, lay_out, paint, rows};
use rdom_tui::{HitTestExt, TuiDom, TuiNodeExt};

/// CSS Display 3 §2.5 with CSS Flexbox §5.4: `order` reorders the items
/// a box-less child hands its flex container, and the box-less element
/// stays on its children's ancestor path (it is still in the DOM, so
/// `:hover` matches it).
#[test]
fn a_contents_wrapper_stays_on_the_hit_path_under_order() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let c = el(&mut dom, f, "span", "c");
    let b = el(&mut dom, c, "b", "");
    let t = dom.create_text_node("x");
    dom.append_child(b, t).unwrap();
    let i = el(&mut dom, f, "i", "o");
    let t = dom.create_text_node("y");
    dom.append_child(i, t).unwrap();
    lay_out(
        &mut dom,
        ".f { display: flex } .c { display: contents } .o { order: -1 }",
        10,
        2,
    );
    // `i` is first (order -1), `b` second.
    assert_eq!(dom.hit_test(1, 0), Some(b));
    let path = dom.hit_test_path(1, 0);
    assert!(path.contains(&c), "{path:?}");
}

/// CSS Flexbox §4: a contiguous run of a flex container's child text —
/// here the text of a box-less child (CSS Display 3 §2.5), with its
/// `::before` / `::after` — is wrapped in an anonymous flex item, so it
/// is laid out and painted.
#[test]
fn the_text_of_a_contents_child_of_a_flex_container_is_painted() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let c = el(&mut dom, f, "span", "c");
    let t = dom.create_text_node("hello");
    dom.append_child(c, t).unwrap();
    let buf = paint(
        &mut dom,
        ".f { display: flex } .c { display: contents } \
         .c::before { content: '<' } .c::after { content: '>' }",
        10,
        2,
    );
    assert_eq!(rows(&buf, 8, 1), ["<hello> "]);
}

/// CSS Overflow 3 §3.1: `overflow` applies to block containers, flex
/// containers and grid containers — a box-less element has no box to
/// clip, so its children are its scroll container ancestor's scrollable
/// overflow — here through an in-flow box they overflow.
#[test]
fn a_contents_element_does_not_clip_its_ancestors_scroll_extent() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    let m = el(&mut dom, s, "div", "m");
    let c = el(&mut dom, m, "div", "c");
    for _ in 0..4 {
        let k = el(&mut dom, c, "div", "");
        let t = dom.create_text_node("row");
        dom.append_child(k, t).unwrap();
    }
    lay_out(
        &mut dom,
        ".s { overflow: auto; height: 2 } .m { height: 1 } \
         .c { display: contents; overflow: hidden }",
        10,
        4,
    );
    assert_eq!(dom.node(s).tui_ext().unwrap().scroll_content_height, 4);
}

/// CSS 2.1 §10.3.7 / §10.6.4 with CSS Display 3 §2.5: the hypothetical
/// box of an absolutely positioned child of a box-less element follows
/// the inline content before it in the parent's line, as it would as
/// the parent's own child.
#[test]
fn the_static_position_inside_a_contents_element_follows_the_line() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let t = dom.create_text_node("Hello");
    dom.append_child(p, t).unwrap();
    let c = el(&mut dom, p, "span", "c");
    let b = el(&mut dom, c, "b", "a");
    let t = dom.create_text_node("X");
    dom.append_child(b, t).unwrap();
    lay_out(
        &mut dom,
        ".c { display: contents } .a { position: absolute }",
        12,
        2,
    );
    let r = dom.node(b).layout_rect().unwrap();
    assert_eq!((r.x, r.y), (5, 0));
}
