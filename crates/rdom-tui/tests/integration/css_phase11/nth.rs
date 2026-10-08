//! C11-NTH — Selectors 4 §13.3–§13.4 `:nth-child()` and its family through a
//! stylesheet, and their restyle when the sibling list changes.

use rdom_tui::{NodeId, TuiDom};

use super::{BLUE, RED, UNSTYLED, app, app_fg, cascade, el, fg};

/// A `ul` with `n` `li` children, classed per `class(i)` (0-based).
fn list(
    dom: &mut TuiDom,
    n: usize,
    class: impl Fn(usize) -> &'static str,
) -> (NodeId, Vec<NodeId>) {
    let root = dom.root();
    let ul = el(dom, root, "ul", "");
    let kids = (0..n).map(|i| el(dom, ul, "li", class(i))).collect();
    (ul, kids)
}

/// Selectors 4 §13.3.1 / §13.3.2 / §13.4.3: zebra rows, the
/// last two, and the first of each type, through the cascade.
#[test]
fn nth_pseudo_classes_style_through_a_sheet() {
    let mut dom = TuiDom::new();
    let (ul, kids) = list(&mut dom, 5, |_| "");
    let p = el(&mut dom, ul, "p", "");
    cascade(
        &mut dom,
        "li:nth-child(2n+1) { color: red } li:nth-last-child(-n+3) { color: blue } \
         :first-of-type { background-color: red }",
    );
    let colors: Vec<_> = kids.iter().map(|&k| fg(&dom, k)).collect();
    assert_eq!(colors, [RED, UNSTYLED, RED, BLUE, BLUE]);
    let bg = |id| {
        use rdom_tui::TuiNodeExt;
        dom.node(id).computed().unwrap().bg
    };
    assert_eq!(bg(kids[0]), RED);
    assert_eq!(bg(kids[1]), rdom_tui::Color::TRANSPARENT);
    assert_eq!(bg(p), RED, "the only `p` is the first of its type");
}

/// Selectors 4 §13.3: inserting or removing a sibling moves the index of
/// the siblings after it (and, for `:nth-last-child`, before it); the
/// next frame restyles them.
#[test]
fn inserting_and_removing_a_sibling_restyles_the_moved_indices() {
    let mut dom = TuiDom::new();
    let (ul, kids) = list(&mut dom, 4, |_| "");
    let mut app = app(
        dom,
        "li:nth-child(odd) { color: red } li:nth-last-child(1) { color: blue }",
    );
    let colors =
        |app: &_, kids: &[NodeId]| kids.iter().map(|&k| app_fg(app, k)).collect::<Vec<_>>();
    assert_eq!(colors(&app, &kids), [RED, UNSTYLED, RED, BLUE]);
    // Insert a new first child: every old child moves one index on.
    let first = app.dom_mut().create_element("li");
    app.dom_mut()
        .insert_before(ul, first, Some(kids[0]))
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, first), RED);
    assert_eq!(colors(&app, &kids), [UNSTYLED, RED, UNSTYLED, BLUE]);
    // Append after the last: the old last is no longer last.
    let last = app.dom_mut().create_element("li");
    app.dom_mut().append_child(ul, last).unwrap();
    app.advance(0).unwrap();
    assert_eq!(colors(&app, &kids), [UNSTYLED, RED, UNSTYLED, RED]);
    assert_eq!(app_fg(&app, last), BLUE);
    // Remove the inserted first child: back to the original indices.
    app.dom_mut().remove_child(ul, first).unwrap();
    app.advance(0).unwrap();
    assert_eq!(colors(&app, &kids), [RED, UNSTYLED, RED, UNSTYLED]);
}

/// Selectors 4 §13.3.1: with `of S`, a sibling starting or stopping to
/// match `S` moves the others' indices — a class change on one element
/// restyles its siblings.
#[test]
fn a_sibling_matching_of_s_or_not_restyles_the_others() {
    let mut dom = TuiDom::new();
    let (_, kids) = list(&mut dom, 4, |_| "x");
    let mut app = app(dom, "li:nth-child(even of .x) { color: red }");
    let colors = |app: &_| kids.iter().map(|&k| app_fg(app, k)).collect::<Vec<_>>();
    assert_eq!(colors(&app), [UNSTYLED, RED, UNSTYLED, RED]);
    app.dom_mut().remove_class(kids[0], "x").unwrap();
    app.advance(0).unwrap();
    assert_eq!(colors(&app), [UNSTYLED, UNSTYLED, RED, UNSTYLED]);
}
