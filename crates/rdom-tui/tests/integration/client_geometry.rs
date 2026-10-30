//! CLIENT-GEOMETRY — `TuiAccessors::client_width` / `client_height` /
//! `client_top` / `client_left` against CSSOM View §6 (`Element.clientTop`,
//! `.clientLeft`, `.clientWidth`, `.clientHeight`): the padding box less
//! the scrollbar gutters, and the border (plus a left-side gutter) before
//! it; 0 for an element with no box or an inline box; the viewport's size
//! for the root.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, NodeId, TuiAccessors, TuiDom, Viewport};

/// `markup` under the root, cascaded with `css` in a `w` × `h` viewport and
/// laid out.
fn laid_out(markup: &str, css: &str, w: u16, h: u16) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).expect("markup parses");
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.set_viewport(Viewport::new(w, h));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, w, h));
    dom
}

fn by_id(dom: &TuiDom, id: &str) -> NodeId {
    dom.get_element_by_id(id)
        .unwrap_or_else(|| panic!("no #{id}"))
}

/// `(clientTop, clientLeft, clientWidth, clientHeight)` of `#id`.
fn client(dom: &TuiDom, id: &str) -> (i32, i32, i32, i32) {
    let n = dom.node(by_id(dom, id));
    (
        n.client_top().expect("an element"),
        n.client_left().expect("an element"),
        n.client_width().expect("an element"),
        n.client_height().expect("an element"),
    )
}

/// §6: with no border and no scrollbar the padding box is the border box,
/// so `clientWidth` / `clientHeight` are its size and the offsets are 0.
#[test]
fn a_plain_box_is_its_border_box() {
    let dom = laid_out(
        "<main><div id=b></div></main>",
        "#b { width: 7; height: 3 }",
        20,
        10,
    );
    assert_eq!(client(&dom, "b"), (0, 0, 7, 3));
}

/// §6: `clientWidth` is the padding box's width — padding counts, the
/// border does not — and `clientTop` / `clientLeft` are the top and left
/// border widths.
#[test]
fn the_border_is_outside_and_the_padding_inside() {
    let dom = laid_out(
        "<main><div id=b></div></main>",
        "#b { box-sizing: border-box; width: 12; height: 6; border: solid; padding: 1 }",
        20,
        10,
    );
    assert_eq!(client(&dom, "b"), (1, 1, 10, 4));
}

/// §6: "excluding the width of any rendered scroll bar" — a scroll
/// container's gutters (CSS Overflow 3 §5.2: between the border and the
/// padding edge) come off the width and the height.
#[test]
fn scrollbar_gutters_are_excluded() {
    let dom = laid_out(
        "<main><div id=b></div></main>",
        "#b { box-sizing: border-box; width: 12; height: 6; border: solid; overflow: scroll }",
        20,
        10,
    );
    assert_eq!(client(&dom, "b"), (1, 1, 9, 3));
}

/// §6 `clientLeft`: the left border plus "the width of the vertical
/// scrollbar placed between the left padding edge and the left border
/// edge" — under `direction: rtl` the bar is on the left.
#[test]
fn a_left_scrollbar_counts_in_client_left() {
    let dom = laid_out(
        "<main><div id=b></div></main>",
        "#b { box-sizing: border-box; width: 12; height: 6; border: solid; \
         overflow-y: scroll; direction: rtl }",
        20,
        10,
    );
    assert_eq!(client(&dom, "b"), (1, 2, 9, 4));
}

/// §6 `clientWidth` agrees with the scroll range: a scroll container's
/// `scrollWidth` less its `clientWidth` is how far it scrolls.
#[test]
fn client_width_is_the_scrollport_the_scroll_range_is_measured_from() {
    let dom = laid_out(
        "<main><div id=b><div id=i></div></div></main>",
        "#b { width: 6; height: 3; overflow: auto } #i { width: 20; height: 10 }",
        30,
        12,
    );
    let n = dom.node(by_id(&dom, "b"));
    let range = n.scroll_range().unwrap();
    assert_eq!(
        n.scroll_width().unwrap() - n.client_width().unwrap(),
        *range.x().end()
    );
    assert_eq!(
        n.scroll_height().unwrap() - n.client_height().unwrap(),
        *range.y().end()
    );
}

/// §6 step 1: "If the element has no associated CSS layout box or if the
/// CSS layout box is inline, return zero" — a non-atomic inline box.
#[test]
fn an_inline_box_is_zero() {
    let dom = laid_out(
        "<main><p><span id=s>some words</span></p></main>",
        "#s { border: solid; padding: 0 1 }",
        30,
        5,
    );
    assert_eq!(client(&dom, "s"), (0, 0, 0, 0));
}

/// An `inline-block` is an atomic inline: a block container inside, so it
/// answers as a block does.
#[test]
fn an_inline_block_answers_as_a_block() {
    let dom = laid_out(
        "<main><p><span id=s>x</span></p></main>",
        "#s { display: inline-block; box-sizing: border-box; width: 6; height: 3; border: solid }",
        30,
        5,
    );
    assert_eq!(client(&dom, "s"), (1, 1, 4, 1));
}

/// §6 step 1: no box — `display: none` (its own or an ancestor's) and
/// `display: contents` — is zero on every reader.
#[test]
fn an_element_with_no_box_is_zero() {
    let dom = laid_out(
        "<main><div id=n></div><div id=h><div id=nested></div></div><div id=c></div></main>",
        "#n, #h { display: none } #c { display: contents } \
         div { width: 5; height: 2; border: solid }",
        30,
        10,
    );
    for id in ["n", "nested", "c"] {
        assert_eq!(client(&dom, id), (0, 0, 0, 0), "#{id}");
    }
}

/// §6: the root element's `clientWidth` / `clientHeight` are the
/// viewport's size; rdom's root is the Fragment that stands for it
/// (DIVERGENCES, C14G-ROOT-ELEMENT), and a terminal viewport has no
/// scrollbar of its own.
#[test]
fn the_root_is_the_viewport() {
    let dom = laid_out("<main id=m></main>", "#m { height: 2 }", 40, 12);
    let root = dom.node(dom.root());
    assert_eq!(root.client_width(), Some(40));
    assert_eq!(root.client_height(), Some(12));
    assert_eq!((root.client_top(), root.client_left()), (Some(0), Some(0)));
}

/// A non-element has no client box.
#[test]
fn a_text_node_has_none() {
    let dom = laid_out("<main id=m>words</main>", "", 20, 5);
    let text = dom.node(by_id(&dom, "m")).first_child().unwrap().id();
    let t = dom.node(text);
    assert_eq!(
        (
            t.client_top(),
            t.client_left(),
            t.client_width(),
            t.client_height()
        ),
        (None, None, None, None)
    );
}

/// `NodeMut` reads the same answers as `NodeRef`, as every accessor does.
#[test]
fn node_mut_reads_the_same() {
    let mut dom = laid_out(
        "<main><div id=b></div></main>",
        "#b { box-sizing: border-box; width: 12; height: 6; border: solid; overflow: scroll }",
        20,
        10,
    );
    let id = by_id(&dom, "b");
    let n = dom.node_mut(id);
    assert_eq!(
        (
            n.client_top(),
            n.client_left(),
            n.client_width(),
            n.client_height()
        ),
        (Some(1), Some(1), Some(9), Some(3))
    );
}

/// An element root (`Dom::with_root_tag`) is the root element too: the
/// viewport's size, whatever its own box.
#[test]
fn an_element_root_is_the_viewport() {
    let mut dom = TuiDom::with_root_tag("html");
    let sheet = rdom_css::from_css_strict(":root { width: 10; height: 3; border: solid }")
        .expect("sheet parses without warnings");
    dom.set_viewport(Viewport::new(30, 8));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 8));
    let root = dom.node(dom.root());
    assert_eq!(
        (root.client_width(), root.client_height()),
        (Some(30), Some(8))
    );
}
