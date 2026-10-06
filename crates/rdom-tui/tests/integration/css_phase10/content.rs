//! C10-CONTENT — CSS Generated Content 3 §2: the full `content` grammar
//! (strings, `counter()` / `counters()`, quotes, `attr()`, `var()`, alt
//! text) on `::before` / `::after`, and `content` on an element.

use rdom_tui::TuiNodeExt;

use super::{el, paint, paint_tree, rows, text_el};

/// CSS Lists 3 §4.3: `counters(name, string)` joins every counter of
/// that name in scope, outermost first, with the string; an optional
/// third argument is the counter style.
#[test]
fn counters_join_the_nested_counters_in_scope() {
    let css = r#".l { counter-reset: s; display: block }
                 .i { display: block }
                 .i::before { counter-increment: s; content: counters(s, ".") " " }
                 .r::before { counter-increment: s; content: counters(s, "-", upper-roman) " " }"#;
    let rows = paint_tree(css, 10, 4, |dom, root| {
        let l = el(dom, root, "div", "l");
        text_el(dom, l, "div", "i", "a");
        let i = text_el(dom, l, "div", "i", "b");
        let inner = el(dom, i, "div", "l");
        text_el(dom, inner, "div", "i", "c");
        text_el(dom, inner, "div", "r", "d");
    });
    assert_eq!(
        rows,
        vec!["1 a       ", "2 b       ", "2.1 c     ", "II-II d   "]
    );
}

/// §2.2 / §2: `open-quote` / `close-quote` insert the marks of the
/// current nesting level and move the depth — the second level takes the
/// second pair (the initial `quotes: auto`, English here: “ ” then ‘ ’).
/// The HTML UA sheet quotes `<q>` (HTML §15.3.6).
#[test]
fn quotes_nest_by_depth_and_q_is_quoted() {
    let rows = paint_tree("", 14, 1, |dom, root| {
        let q = dom.create_element("q");
        dom.append_child(root, q).unwrap();
        let a = dom.create_text_node("a ");
        dom.append_child(q, a).unwrap();
        let inner = text_el(dom, q, "q", "", "b");
        let _ = inner;
        let c = dom.create_text_node(" c");
        dom.append_child(q, c).unwrap();
    });
    assert_eq!(rows, vec!["“a ‘b’ c”     "]);
}

/// §2.2: "A close-quote … that would make the quote depth negative is in
/// error and is ignored": no mark, the depth stays 0. `no-open-quote`
/// raises the depth without a mark, so the next `open-quote` takes the
/// second level's mark.
#[test]
fn a_close_quote_at_depth_zero_is_ignored_and_no_open_quote_nests() {
    let css = r#".c::before { content: close-quote "x" }
                 .n::before { content: no-open-quote }
                 .o::before { content: open-quote }"#;
    let rows = paint_tree(css, 6, 3, |dom, root| {
        text_el(dom, root, "div", "c", "1");
        text_el(dom, root, "div", "n", "2");
        text_el(dom, root, "div", "o", "3");
    });
    assert_eq!(rows, vec!["x1    ", "2     ", "‘3    "]);
}

/// §2.2: the quote depth runs across the document in tree order, not
/// down the element tree: an `open-quote` in one paragraph is closed by
/// a `close-quote` in a later one, and the paragraph between them nests.
#[test]
fn quote_depth_runs_in_tree_order_across_elements() {
    let css = r#".o::before { content: open-quote }
                 .c::after { content: close-quote }"#;
    let rows = paint_tree(css, 6, 3, |dom, root| {
        text_el(dom, root, "div", "o", "a");
        text_el(dom, root, "div", "o", "b");
        let c = text_el(dom, root, "div", "c", "c");
        let _ = c;
    });
    assert_eq!(rows, vec!["“a    ", "‘b    ", "c’    "]);
}

/// CSS Variables 1 §3: `var()` substitutes into `content` like any
/// value, between other items.
#[test]
fn var_substitutes_inside_content() {
    let css = r#".p { --x: "zz" }
                 .p::before { content: "a" var(--x) "b" }"#;
    let rows = paint_tree(css, 8, 1, |dom, root| {
        text_el(dom, root, "div", "p", "t");
    });
    assert_eq!(rows, vec!["azzbt   "]);
}

/// §2: the alt text after `/` is the content's alternative for speech
/// and other non-visual media — it is not painted; the computed style
/// keeps it resolved (`content_alt`), counters and `attr()` included.
#[test]
fn alt_text_is_kept_and_not_painted() {
    let mut dom = rdom_tui::TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "div", "p", "t");
    dom.set_attribute(p, "data-n", "five").unwrap();
    let css = r#".p { counter-reset: n 4 }
                 .p::before { content: "★" / "star " attr(data-n) " " counter(n) }"#;
    let buf = paint(&mut dom, css, 4, 1);
    assert_eq!(rows(&buf, 4, 1), vec!["★t  "]);
    let before = dom.node(p).computed_before().expect("a ::before box");
    assert_eq!(before.content.as_deref(), Some("★"));
    assert_eq!(before.content_alt.as_deref(), Some("star five 4"));
}

/// §2 gives `<content-list>` on an element the power to replace its
/// contents; no engine implements that (Chromium, Gecko and WebKit
/// replace an element only with an image, `<content-replacement>`),
/// and rdom follows them: a string `content` on an element leaves its
/// children rendered (DIVERGENCES §2).
#[test]
fn a_string_content_on_an_element_does_not_replace_its_children() {
    let rows = paint_tree(r#".p { content: "x" }"#, 4, 1, |dom, root| {
        text_el(dom, root, "div", "p", "ab");
    });
    assert_eq!(rows, vec!["ab  "]);
}

/// An element's own `content` does not take part in the quote depth —
/// it generates nothing — so a later `::before` still opens at level 1.
#[test]
fn an_elements_own_quote_content_moves_no_depth() {
    let css = r#".e { content: open-quote } .o::before { content: open-quote }"#;
    let rows = paint_tree(css, 4, 2, |dom, root| {
        text_el(dom, root, "div", "e", "a");
        text_el(dom, root, "div", "o", "b");
    });
    assert_eq!(rows, vec!["a   ", "“b  "]);
}
