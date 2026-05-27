//! Regression: a flex container with `<span>` (or other
//! `display: inline`) children must blockify them into flex items
//! and run flex layout normally — NOT route through the IFC path
//! and give them zero-sized layout rects.
//!
//! Per CSS Flexbox §3 "Flex Items": *"The computed display value
//! of a child of a flex container is blockified."* A `<span>`
//! inside `display: flex` becomes a flex item.
//!
//! Surfaced by the showcase status bar's two-slot pattern (hints
//! left + mouse-position right): a `<footer>` with `display: flex`
//! containing two `<span>` children was producing zero-sized rects
//! for both spans and the mouse-pos display never appeared.

use rdom_tui::layout::{Direction, Flow};
use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, Stylesheet, TuiDom, TuiNodeExt, TuiStyle};

#[test]
fn flex_container_with_inline_children_gives_them_real_rects() {
    // <footer class="bar">                  ← display: flex
    //   <span class="left"></span>          ← UA: display: inline
    //   <span class="right"></span>         ← UA: display: inline
    // </footer>
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let bar = dom.create_element("footer");
    let left = dom.create_element("span");
    let right = dom.create_element("span");
    dom.append_child(root, bar).unwrap();
    dom.append_child(bar, left).unwrap();
    dom.append_child(bar, right).unwrap();

    // `display: flex; flex-direction: row` on the parent. Children
    // have their UA `display: inline` — the cascade keeps that
    // value, but the flex parent must BLOCKIFY them for layout per
    // CSS Flexbox §3.
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "footer",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(rdom_tui::layout::Size::Fixed(40))
                .height(rdom_tui::layout::Size::Fixed(1)),
        )
        .rule_unchecked(
            ".left",
            TuiStyle::new()
                .width(rdom_tui::layout::Size::Fixed(10))
                .height(rdom_tui::layout::Size::Fixed(1)),
        )
        .rule_unchecked(
            ".right",
            TuiStyle::new()
                .width(rdom_tui::layout::Size::Fixed(10))
                .height(rdom_tui::layout::Size::Fixed(1)),
        );
    dom.set_attribute(left, "class", "left").unwrap();
    dom.set_attribute(right, "class", "right").unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 1));

    let left_rect = dom.node(left).layout_rect().expect("left laid out");
    let right_rect = dom.node(right).layout_rect().expect("right laid out");

    // BEFORE the fix: both rects were `LayoutRect { x: 0, y: 0, width: 0, height: 0 }`
    // because the IFC path zeroed every child. AFTER the fix: each
    // child gets its declared 10x1 rect, and the right child is
    // positioned after the left.
    assert_eq!(
        left_rect.width, 10,
        "left.width must respect declared 10 cells"
    );
    assert_eq!(
        right_rect.width, 10,
        "right.width must respect declared 10 cells"
    );
    assert!(
        right_rect.x >= left_rect.x + left_rect.width as i32,
        "right must be positioned AFTER left in a row-flex layout; \
         got left={left_rect:?} right={right_rect:?}"
    );
}

#[test]
fn flex_container_with_inline_child_and_text_does_not_become_ifc() {
    // Edge case: `<div display:flex>` with an inline `<span>` AND a
    // direct text node sibling. Without the blockification carve-
    // out, the parent was being treated as an IFC because of the
    // inline span. With the fix, the flex parent stays a flex
    // container; the text node and span both flex-item up.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let container = dom.create_element("div");
    let span = dom.create_element("span");
    dom.append_child(root, container).unwrap();
    dom.append_child(container, span).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(rdom_tui::layout::Size::Fixed(20))
                .height(rdom_tui::layout::Size::Fixed(1)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .width(rdom_tui::layout::Size::Fixed(5))
                .height(rdom_tui::layout::Size::Fixed(1)),
        );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 1));

    let span_rect = dom.node(span).layout_rect().expect("span laid out");
    assert_eq!(
        span_rect.width, 5,
        "inline span inside flex container must keep its declared width"
    );
}
