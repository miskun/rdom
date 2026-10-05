//! C6G-ANON-FLEX-ITEMS — anonymous flex items (CSS Flexbox §4): each
//! contiguous run of a flex container's child text is wrapped in an
//! anonymous block container flex item (whitespace-only runs are not
//! rendered), and its `::before` / `::after` are flex items of their
//! own — laid out, painted, hit-tested, measured, ordered, aligned and
//! wrapped like any other item.

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::render::{InlineFlow, inline_flow_for_text};
use rdom_tui::{HitTestExt, TuiDom};

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, data: &str) -> rdom_tui::NodeId {
    let t = dom.create_text_node(data);
    dom.append_child(parent, t).unwrap();
    t
}

/// CSS Flexbox §4: "each contiguous sequence of child text runs is
/// wrapped in an anonymous block container flex item" — so the text
/// beside an element item is an item before it.
#[test]
fn text_beside_an_element_item_is_an_anonymous_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "ab");
    let s = el(&mut dom, f, "span", "");
    text(&mut dom, s, "hello");
    let buf = paint(&mut dom, ".f { display: flex }", 10, 1);
    assert_eq!(rows(&buf, 10, 1), ["abhello   "]);
    assert_eq!(rect(&dom, s).x, 2);
}

/// CSS Flexbox §4: the `::before` / `::after` of a flex container are
/// flex items, and a gap separates every pair of items (CSS Box
/// Alignment 3 §8.1); a whitespace-only run is not rendered, so it is
/// no item and takes no gap.
#[test]
fn pseudo_elements_and_text_runs_are_items_apart() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "ab");
    let s = el(&mut dom, f, "span", "");
    text(&mut dom, s, "x");
    text(&mut dom, f, "  ");
    let t = el(&mut dom, f, "span", "");
    text(&mut dom, t, "y");
    let buf = paint(
        &mut dom,
        ".f { display: flex; gap: 1 } .f::before { content: '[' } .f::after { content: ']' }",
        12,
        1,
    );
    assert_eq!(rows(&buf, 12, 1), ["[ ab x y ]  "]);
}

/// CSS Flexbox §5.4: an anonymous item has `order: 0`, so an item with
/// a negative `order` comes before it.
#[test]
fn an_anonymous_item_has_order_zero() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "ab");
    let s = el(&mut dom, f, "span", "o");
    text(&mut dom, s, "x");
    let buf = paint(&mut dom, ".f { display: flex } .o { order: -1 }", 6, 1);
    assert_eq!(rows(&buf, 6, 1), ["xab   "]);
}

/// CSS Flexbox §8.2 / §8.3: `justify-content` and `align-items` place a
/// container's only anonymous item like any other — `text-align` does
/// not apply to the container's text, which its item holds.
#[test]
fn a_text_only_container_aligns_its_anonymous_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "hi");
    let buf = paint(
        &mut dom,
        ".f { display: flex; justify-content: center; align-items: flex-end; height: 3 }",
        10,
        3,
    );
    assert_eq!(
        rows(&buf, 10, 3),
        ["          ", "          ", "    hi    "]
    );
}

/// CSS Flexbox §9.3: an anonymous item takes part in line breaking at
/// its outer hypothetical main size.
#[test]
fn anonymous_items_wrap_onto_lines() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "abcd");
    let s = el(&mut dom, f, "span", "");
    text(&mut dom, s, "efgh");
    let buf = paint(
        &mut dom,
        ".f { display: flex; flex-wrap: wrap; width: 6 }",
        6,
        2,
    );
    assert_eq!(rows(&buf, 6, 2), ["abcd  ", "efgh  "]);
}

/// CSS Flexbox §9.9.1: a flex container's max-content main size sums
/// its items' contributions, anonymous ones and pseudo-elements
/// included, with the gaps between them.
#[test]
fn anonymous_items_count_in_the_intrinsic_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let col = el(&mut dom, root, "div", "col");
    let f = el(&mut dom, col, "div", "f");
    text(&mut dom, f, "ab");
    let s = el(&mut dom, f, "span", "");
    text(&mut dom, s, "cd");
    lay_out(
        &mut dom,
        ".col { display: flex; flex-direction: column; align-items: flex-start } \
         .f { display: flex; gap: 1 } .f::after { content: '!' }",
        20,
        2,
    );
    // "ab" + gap + "cd" + gap + "!".
    assert_eq!(rect(&dom, f).width, 7);
}

/// Hit-testing and the caret reach an anonymous item's text: a point on
/// it resolves to a position in its text node, the one beside it on the
/// same line to the other's (the items share the line's rows).
#[test]
fn the_caret_resolves_into_each_anonymous_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ab = text(&mut dom, f, "ab");
    let s = el(&mut dom, f, "span", "");
    text(&mut dom, s, "x");
    let cd = text(&mut dom, f, "cd");
    lay_out(&mut dom, ".f { display: flex }", 10, 1);
    let at = dom.position_at(1, 0).expect("a position on `ab`");
    assert_eq!((at.node, at.offset), (ab, 1));
    let at = dom.position_at(4, 0).expect("a position on `cd`");
    assert_eq!((at.node, at.offset), (cd, 1));
    assert_eq!(
        inline_flow_for_text(&dom, cd),
        Some(InlineFlow::Anonymous {
            container: f,
            index: 1
        })
    );
}

/// CSS Display 3 §2.5 with CSS Flexbox §4: a box-less child's text
/// joins the container's runs, and its `::before` / `::after` are child
/// boxes of the container — items of their own, a gap apart.
#[test]
fn a_contents_childs_pseudo_elements_are_items_of_their_own() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    text(&mut dom, f, "a");
    let c = el(&mut dom, f, "span", "c");
    text(&mut dom, c, "b");
    text(&mut dom, f, "c");
    let buf = paint(
        &mut dom,
        ".f { display: flex; gap: 1 } .c { display: contents } \
         .c::before { content: '<' } .c::after { content: '>' }",
        12,
        1,
    );
    assert_eq!(rows(&buf, 12, 1), ["a < b > c   "]);
}
