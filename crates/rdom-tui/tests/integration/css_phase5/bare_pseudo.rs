//! C5G-BARE-PSEUDO — a pseudo-element with no compound before it.
//! Selectors 4 §5.2: when a compound has no type selector, the
//! universal selector is implied, so `::before` is `*::before` and
//! `div ::before` is `div *::before` (the pseudo-element of every
//! descendant, not of the `div`).

use super::{el, paint, rows};
use rdom_tui::{Color, TuiDom};

/// The Tailwind preflight / modern-normalize reset shape: a list
/// holding bare `::before` / `::after` items. The whole rule applies —
/// before C5G-BARE-PSEUDO it was dropped (strict parse failed).
#[test]
fn a_list_with_bare_pseudo_elements_applies_to_every_pseudo_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("z");
    dom.append_child(b, t).unwrap();
    let buf = paint(
        &mut dom,
        ".none, ::before, ::after { background-color: rgb(255 0 0) }
         .b::before { content: \"x\" } .b::after { content: \"y\" }",
        4,
        1,
    );
    assert_eq!(rows(&buf, 4, 1), vec!["xzy "]);
    let red = Color::Rgb(255, 0, 0);
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        red,
        "::before took the bare item"
    );
    assert_ne!(
        buf.cell(1, 0).unwrap().bg,
        red,
        "the element itself did not"
    );
    assert_eq!(
        buf.cell(2, 0).unwrap().bg,
        red,
        "::after took the bare item"
    );
}

/// `div ::before` (descendant combinator) is `div *::before`: the
/// `::before` of the `div`'s descendants, not of the `div`.
#[test]
fn a_descendant_bare_pseudo_element_matches_the_descendants() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    let t = dom.create_text_node("A");
    dom.append_child(a, t).unwrap();
    let span = el(&mut dom, a, "span", "");
    let t = dom.create_text_node("B");
    dom.append_child(span, t).unwrap();
    let buf = paint(&mut dom, ".a ::before { content: \">\" }", 4, 1);
    assert_eq!(rows(&buf, 4, 1), vec!["A>B "]);
}

/// `div > ::after` is `div > *::after`: the children's `::after`.
#[test]
fn a_child_combinator_bare_pseudo_element_matches_the_children() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    let span = el(&mut dom, a, "span", "");
    let t = dom.create_text_node("B");
    dom.append_child(span, t).unwrap();
    let t = dom.create_text_node("C");
    dom.append_child(a, t).unwrap();
    let buf = paint(&mut dom, ".a > ::after { content: \"<\" }", 4, 1);
    assert_eq!(rows(&buf, 4, 1), vec!["B<C "]);
}
