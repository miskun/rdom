//! C10-LIST-ITEM — CSS Lists 3: `display: list-item` generates a
//! `::marker` (§3.1–§3.2) whose content `list-style-type` makes (§3.4),
//! placed by `list-style-position` (§3.5) and `marker-side` (§3.6).

use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

use super::{el, lay_out, text_el};

/// The `::marker` text of `id`, `None` without a marker.
fn marker(dom: &TuiDom, id: NodeId) -> Option<String> {
    dom.node(id)
        .computed_marker()
        .and_then(|m| m.content.clone())
}

/// Lay out `css` over a `<ul>` / `<ol>` (`tag`) of items `texts` in a
/// `w` × `h` viewport: the dom and the items.
fn list(tag: &str, css: &str, texts: &[&str], w: u16, h: u16) -> (TuiDom, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let l = el(&mut dom, root, tag, "l");
    let items = texts
        .iter()
        .map(|t| text_el(&mut dom, l, "li", "", t))
        .collect();
    lay_out(&mut dom, css, w, h);
    (dom, items)
}

/// §3.1: a list item generates a `::marker`; §3.4: its content is the
/// `list-item` counter in `list-style-type`, between the counter style's
/// prefix and suffix — HTML's `ul` is `disc`, `ol` `decimal` (HTML
/// §15.3.8) — a string as written, nothing for `none`.
#[test]
fn list_style_type_makes_the_marker() {
    let (dom, items) = list("ol", "", &["a", "b"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("1. "));
    assert_eq!(marker(&dom, items[1]).as_deref(), Some("2. "));
    let (dom, items) = list("ul", "", &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("• "));
    let (dom, items) = list(
        "ol",
        "li { list-style-type: upper-roman }",
        &["a", "b"],
        20,
        4,
    );
    assert_eq!(marker(&dom, items[1]).as_deref(), Some("II. "));
    let (dom, items) = list("ul", r#"li { list-style-type: "→ " }"#, &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("→ "));
    let (dom, items) = list("ul", "ul { list-style: none }", &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]), None);
}

/// HTML §15.3.8: a `ul` nested in a list is `circle`, one level deeper
/// `square`.
#[test]
fn nested_bullet_lists_change_their_bullet() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, ul, "li", "", "a");
    let ul2 = el(&mut dom, li, "ul", "");
    let li2 = text_el(&mut dom, ul2, "li", "", "b");
    let ul3 = el(&mut dom, li2, "ul", "");
    let li3 = text_el(&mut dom, ul3, "li", "", "c");
    lay_out(&mut dom, "", 20, 6);
    assert_eq!(marker(&dom, li).as_deref(), Some("• "));
    assert_eq!(marker(&dom, li2).as_deref(), Some("◦ "));
    assert_eq!(marker(&dom, li3).as_deref(), Some("▪ "));
}

/// §3.2: `::marker { content }` replaces the marker; `content: none`
/// removes it, `normal` is `list-style-type`'s. Properties outside the
/// marker's list do not apply (`padding`), `color` does.
#[test]
fn the_marker_pseudo_element() {
    let (dom, items) = list(
        "ol",
        r#"li::marker { content: "*" counter(list-item) ") "; padding: 3; color: red }
           li + li::marker { content: none }"#,
        &["a", "b"],
        20,
        4,
    );
    let m = dom.node(items[0]).computed_marker().unwrap();
    assert_eq!(m.content.as_deref(), Some("*1) "));
    assert_eq!(m.padding.top, rdom_tui::layout::PaddingValue::Cells(0));
    assert_eq!(m.fg, rdom_tui::Color::Rgb(255, 0, 0));
    assert_eq!(marker(&dom, items[1]), None);
}

/// §3.1: any `display: list-item` box has a marker — `disc` by default
/// (the initial `list-style-type`); an `li` made `display: block` has
/// none.
#[test]
fn display_list_item_generates_the_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = text_el(&mut dom, root, "div", "d", "a");
    let l = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, l, "li", "b", "b");
    lay_out(
        &mut dom,
        ".d { display: list-item } .b { display: block }",
        20,
        4,
    );
    assert_eq!(marker(&dom, d).as_deref(), Some("• "));
    assert_eq!(marker(&dom, li), None);
}

/// CSS Lists 3 §3.2's UA sheet: `::marker { text-transform: none }` —
/// an uppercased item keeps a lowercase alphabetic marker.
#[test]
fn the_marker_is_not_transformed() {
    let (dom, items) = list(
        "ol",
        "li { list-style-type: lower-alpha; text-transform: uppercase }",
        &["a"],
        20,
        4,
    );
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("a. "));
}
