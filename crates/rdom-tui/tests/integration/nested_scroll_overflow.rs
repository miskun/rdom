//! CSS Overflow 3 §2.2: a scroll container's scrollable overflow area
//! covers its in-flow descendants' boxes, but a descendant that clips
//! its own content (an intermediate scroll container) contributes only
//! its own border box — whatever overflows *it* is its own business.
//!
//! The regression these pin (`SCROLL-OVERFLOW-NESTED-ANON-1`): the
//! anonymous block boxes inside a nested scroll container (the inline
//! runs between its block children) leaked into the outer container's
//! scroll size, so the outer pane grew a scrollbar and
//! `scrollIntoView()` on an inner child scrolled the outer pane too.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, NodeId, Stylesheet, TuiAccessors, TuiAccessorsMut, TuiDom};

const CSS: &str = r#"
.outer { display: block; width: 30; height: 10; overflow-y: auto; }
.inner { display: block; height: 4; overflow-y: auto; }
p { display: block; }
"#;

fn layout(dom: &mut TuiDom, sheet: &Stylesheet) {
    dom.cascade(sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 20));
}

/// `.outer > .inner > p × 12`. With `separator = ""` the inline runs
/// between the paragraphs are pure whitespace; with a word they hold
/// real text and so keep their anonymous boxes.
fn fixture(separator: &str) -> (TuiDom, Stylesheet, NodeId, NodeId, NodeId) {
    let mut markup = String::from("<div class=\"outer\">\n  <div class=\"inner\">\n");
    for i in 1..=12 {
        markup.push_str(&format!("    <p>entry {i:02}</p>\n    {separator}\n"));
    }
    markup.push_str("  </div>\n</div>");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, &markup, root).expect("markup parses");
    let outer = dom.node(root).query_selector(".outer").unwrap().id();
    let inner = dom.node(root).query_selector(".inner").unwrap().id();
    let last = dom.node(inner).last_element_child().unwrap().id();
    let sheet = rdom_css::from_css(CSS);
    layout(&mut dom, &sheet);
    (dom, sheet, outer, inner, last)
}

fn scroll_height(dom: &TuiDom, id: NodeId) -> usize {
    dom.node(id).ext().unwrap().scroll_content_height
}

#[test]
fn nested_scroll_container_whitespace_runs_do_not_grow_the_outer_scroll_size() {
    let (dom, _, outer, inner, _) = fixture("");
    assert!(
        scroll_height(&dom, inner) > 4,
        "the inner box really overflows (got {})",
        scroll_height(&dom, inner)
    );
    assert_eq!(
        scroll_height(&dom, outer),
        10,
        "the outer pane's area is its 10-row scrollport: the inner box's 4 rows fit, \
         its hidden content does not count"
    );
}

#[test]
fn nested_scroll_container_text_runs_do_not_grow_the_outer_scroll_size() {
    let (dom, _, outer, inner, _) = fixture("between");
    assert!(scroll_height(&dom, inner) > 12, "entries plus text runs");
    assert_eq!(scroll_height(&dom, outer), 10, "the scrollport");
}

#[test]
fn scrolled_nested_scroll_container_does_not_grow_the_outer_scroll_size() {
    let (mut dom, sheet, outer, inner, _) = fixture("between");
    dom.node_mut(inner).set_scroll_top(6).unwrap();
    layout(&mut dom, &sheet);
    assert_eq!(dom.node(inner).scroll_top(), Some(6));
    assert_eq!(scroll_height(&dom, outer), 10, "the scrollport");
}

#[test]
fn scroll_into_view_in_a_nested_container_leaves_an_unscrollable_outer_still() {
    for separator in ["", "between"] {
        let (mut dom, sheet, outer, inner, last) = fixture(separator);
        dom.node_mut(last).scroll_into_view().unwrap();
        layout(&mut dom, &sheet);
        assert!(
            dom.node(inner).scroll_top().unwrap() > 0,
            "the inner box scrolls its last entry into view ({separator:?})"
        );
        assert_eq!(
            dom.node(outer).scroll_top(),
            Some(0),
            "the outer pane has nothing to scroll ({separator:?})"
        );
    }
}
