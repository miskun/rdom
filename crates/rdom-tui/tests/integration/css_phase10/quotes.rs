//! C10-QUOTES — CSS Generated Content 3 §2.1: `quotes` chooses the
//! marks `open-quote` / `close-quote` insert, per nesting level.

use super::{el, paint_tree, text_el};
use rdom_tui::{NodeId, TuiDom};

/// `<q>` holding `a `, an inner `<q>` holding `b`, then ` c`: the
/// inner `<q>`.
fn nested_q(dom: &mut TuiDom, parent: NodeId) -> NodeId {
    let q = el(dom, parent, "q", "");
    let a = dom.create_text_node("a ");
    dom.append_child(q, a).unwrap();
    let inner = text_el(dom, q, "q", "", "b");
    let c = dom.create_text_node(" c");
    dom.append_child(q, c).unwrap();
    inner
}

/// §2.1: `[<string> <string>]+` — the first pair quotes the outermost
/// level, the next the level inside it; a level past the last pair uses
/// the last pair.
#[test]
fn author_pairs_quote_each_level() {
    let css = r#"q { quotes: "<" ">" "{" "}" }"#;
    let rows = paint_tree(css, 16, 1, |dom, root| {
        // A third level inside the inner `<q>`.
        let inner = nested_q(dom, root);
        text_el(dom, inner, "q", "", "d");
    });
    assert_eq!(rows, vec!["<a {b{d}} c>    "]);
}

/// §2.1 `none`: `open-quote` / `close-quote` produce no mark — but the
/// depth still moves (§2.2), so the next `<q>` after an unquoted one
/// nests inside it.
#[test]
fn quotes_none_inserts_no_mark_but_moves_the_depth() {
    let css = r#".n { quotes: none } .o::before { content: open-quote }"#;
    let rows = paint_tree(css, 8, 2, |dom, root| {
        let n = el(dom, root, "div", "n");
        let o = el(dom, n, "div", "o");
        let t = dom.create_text_node("x");
        dom.append_child(o, t).unwrap();
        text_el(dom, root, "div", "o", "y");
    });
    assert_eq!(rows, vec!["x       ", "‘y      "]);
}

/// §2.1 `auto`: the marks of the element's content language (its own or
/// nearest ancestor's `lang`, HTML §3.2.6.2): German „…“ / ‚…‘, French
/// «…», Finnish ”…”, Japanese 「…」 / 『…』; an unknown language falls
/// back to English.
#[test]
fn auto_quotes_follow_the_content_language() {
    let rows = paint_tree("div { display: block }", 14, 5, |dom, root| {
        for lang in ["de", "fr", "fi", "ja", "x-klingon"] {
            let d = el(dom, root, "div", "");
            dom.set_attribute(d, "lang", lang).unwrap();
            nested_q(dom, d);
        }
    });
    assert_eq!(
        rows,
        vec![
            "„a ‚b‘ c“     ",
            "«a «b» c»     ",
            "”a ’b’ c”     ",
            "「a 『b』 c」 ",
            "“a ‘b’ c”     ",
        ]
    );
}

/// The language is the element's own: an English `<q>` inside German
/// text takes English marks, and its `lang` reaches its `::after` too.
#[test]
fn auto_quotes_use_the_quoting_elements_own_language() {
    let rows = paint_tree("", 10, 1, |dom, root| {
        let d = el(dom, root, "div", "");
        dom.set_attribute(d, "lang", "de").unwrap();
        let q = text_el(dom, d, "q", "", "a");
        dom.set_attribute(q, "lang", "en").unwrap();
    });
    assert_eq!(rows, vec!["“a”       "]);
}

/// §2.1 `match-parent`: "the same quotation mark system as the parent" —
/// a French `<q>` under a German parent keeps German marks.
#[test]
fn match_parent_takes_the_parents_marks() {
    let css = "q.m { quotes: match-parent }";
    let rows = paint_tree(css, 10, 1, |dom, root| {
        let d = el(dom, root, "div", "");
        dom.set_attribute(d, "lang", "de").unwrap();
        let q = text_el(dom, d, "q", "m", "a");
        dom.set_attribute(q, "lang", "fr").unwrap();
    });
    assert_eq!(rows, vec!["„a“       "]);
}
