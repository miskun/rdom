//! C6-DISPLAY-KEYWORDS — `display: contents`, `flow-root` and the
//! multi-keyword syntax (CSS Display 3 §2) in layout, paint, hit
//! testing and focus.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::{HitTestExt, TuiDom, TuiNodeExt};

/// CSS Display 3 §2.5: a `display: contents` element generates no box
/// — its padding, border and background are not drawn — and its
/// children are laid out as children of its parent: here block boxes
/// in the parent's block flow, at the parent's content edge, and its
/// text in an anonymous block box of the parent (CSS 2.1 §9.2.1.1).
#[test]
fn contents_children_join_the_parents_block_flow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let c = el(&mut dom, p, "div", "c");
    let t = dom.create_text_node("cc");
    dom.append_child(c, t).unwrap();
    let x = el(&mut dom, c, "div", "x");
    let t = dom.create_text_node("xx");
    dom.append_child(x, t).unwrap();
    let y = el(&mut dom, p, "div", "y");
    let t = dom.create_text_node("yy");
    dom.append_child(y, t).unwrap();
    let buf = paint(
        &mut dom,
        ".p { width: 8 } \
         .c { display: contents; padding: 1; border: solid; margin: 2; background-color: red }",
        10,
        4,
    );
    assert_eq!((rect(&dom, x).x, rect(&dom, x).y), (0, 1));
    assert_eq!((rect(&dom, y).x, rect(&dom, y).y), (0, 2));
    assert_eq!(rows(&buf, 4, 3), ["cc  ", "xx  ", "yy  "]);
    for x in 0..4 {
        assert_eq!(
            buf.cell(x, 0).unwrap().bg,
            rdom_tui::Color::Reset,
            "({x}, 0)"
        );
    }
}

/// CSS Display 3 §2.5 with Flexbox §4: the children of a `contents`
/// child of a flex container are its flex items.
#[test]
fn contents_children_are_flex_items() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "i");
    let c = el(&mut dom, f, "div", "c");
    let b = el(&mut dom, c, "div", "i");
    let d = el(&mut dom, c, "div", "i");
    lay_out(
        &mut dom,
        ".f { display: flex; flex-direction: row; width: 9 } .c { display: contents } \
         .i { width: 3; height: 1; flex-shrink: 0 }",
        10,
        3,
    );
    let xs: Vec<i32> = [a, b, d].iter().map(|&n| rect(&dom, n).x).collect();
    assert_eq!(xs, [0, 3, 6]);
    assert_eq!(rect(&dom, b).y, 0);
}

/// CSS Display 3 §2.5: inherited properties still inherit through the
/// box-less element, and its `::before` / `::after` are generated as
/// its first / last children — inline boxes in the parent's line.
#[test]
fn contents_inherits_and_generates_its_pseudos() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    let t = dom.create_text_node("a");
    dom.append_child(p, t).unwrap();
    let c = el(&mut dom, p, "span", "c");
    let t = dom.create_text_node("b");
    dom.append_child(c, t).unwrap();
    let t = dom.create_text_node("c");
    dom.append_child(p, t).unwrap();
    let buf = paint(
        &mut dom,
        ".c { display: contents; color: red } \
         .c::before { content: '<' } .c::after { content: '>' }",
        10,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["a<b>c "]);
    assert_eq!(buf.cell(2, 0).unwrap().fg, rdom_tui::Color::Rgb(255, 0, 0));

    // The same for a box-less element that holds a block box, in a
    // block container: the pseudos are inline boxes before and after
    // its children (CSS 2.1 §9.2.1.1: each on an anonymous line).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "");
    let c = el(&mut dom, p, "div", "c");
    let x = el(&mut dom, c, "div", "");
    let t = dom.create_text_node("x");
    dom.append_child(x, t).unwrap();
    let buf = paint(
        &mut dom,
        ".c { display: contents } .c::before { content: '<' } .c::after { content: '>' }",
        10,
        3,
    );
    assert_eq!(rows(&buf, 2, 3), ["< ", "x ", "> "]);
}

/// CSS Display 3 §2.5: the box-less element's children are hit where
/// they are, with it on their ancestor path (it is still in the DOM);
/// the element itself, having no box, is never the target.
#[test]
fn contents_children_hit_test_through_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let c = el(&mut dom, p, "div", "c");
    let x = el(&mut dom, c, "div", "x");
    let t = dom.create_text_node("xxxx");
    dom.append_child(x, t).unwrap();
    lay_out(
        &mut dom,
        ".p { width: 8 } .c { display: contents } .x { width: 4 }",
        10,
        3,
    );
    assert_eq!(dom.hit_test(1, 0), Some(x));
    let path = dom.hit_test_path(1, 0);
    assert!(path.contains(&c), "{path:?}");
    assert_eq!(dom.hit_test(6, 0), Some(p));
}

/// HTML §6.6.3 ("being rendered" includes delegating rendering to its
/// children) — Chromium focuses a `display: contents` button since
/// 2023; CSS Display 3 Appendix B: `display: contents` on a form
/// control such as `<input>` computes to `display: none`, which is not
/// focusable.
#[test]
fn a_contents_button_stays_focusable_and_a_contents_input_is_none() {
    use rdom_tui::runtime::focus::tabindex::focusable_elements;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "button", "c");
    let t = dom.create_text_node("go");
    dom.append_child(b, t).unwrap();
    let i = el(&mut dom, root, "input", "c");
    lay_out(&mut dom, ".c { display: contents }", 10, 3);
    assert_eq!(focusable_elements(&dom), [b]);
    assert_eq!(
        dom.node(i).computed().unwrap().display,
        rdom_tui::Display::None
    );
}

/// CSS Display 3 §2.2 (`flow-root`) with CSS 2.1 §8.3.1 / §9.4.1: a
/// flow-root box establishes a block formatting context, so its first
/// child's top margin stays inside it instead of collapsing through.
#[test]
fn flow_root_keeps_its_childs_margin_inside() {
    for (display, (p_y, k_y)) in [("block", (2, 2)), ("flow-root", (0, 2))] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let w = el(&mut dom, root, "div", "w");
        let p = el(&mut dom, w, "div", "p");
        let k = el(&mut dom, p, "div", "k");
        lay_out(
            &mut dom,
            &format!(".p {{ display: {display} }} .k {{ margin-top: 2; height: 1 }}"),
            10,
            5,
        );
        assert_eq!((rect(&dom, p).y, rect(&dom, k).y), (p_y, k_y), "{display}");
    }
}

/// CSS Display 3 §2.7: the multi-keyword forms lay out as their legacy
/// keywords — `block flex` is `flex`, `inline flow-root` is
/// `inline-block`.
#[test]
fn multi_keyword_forms_lay_out_as_their_legacy_keywords() {
    for (css, legacy) in [("block flex", "flex"), ("inline flow-root", "inline-block")] {
        let lay = |display: &str| {
            let mut dom = TuiDom::new();
            let root = dom.root();
            let p = el(&mut dom, root, "p", "");
            let t = dom.create_text_node("ab ");
            dom.append_child(p, t).unwrap();
            let f = el(&mut dom, p, "span", "f");
            for _ in 0..2 {
                let i = el(&mut dom, f, "span", "i");
                let t = dom.create_text_node("x");
                dom.append_child(i, t).unwrap();
            }
            lay_out(
                &mut dom,
                &format!(".f {{ display: {display}; width: 6; padding: 0 1 }}"),
                12,
                3,
            );
            rect(&dom, f)
        };
        assert_eq!(lay(css), lay(legacy), "{css}");
    }
}
