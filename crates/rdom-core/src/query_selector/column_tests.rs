//! The column combinator and `:nth-col()` / `:nth-last-col()` (Selectors 4
//! §16.1–§16.3, C13-COLUMN), from HTML's table model (HTML §4.9.12.1):
//! `<col>` / `<colgroup>` columns, `colspan` / `rowspan` cells.

use crate::selectors::{self, Combinator, SimpleSelector};
use crate::{Dom, NodeId, SelectorCaches};

/// `<tag attr=value…>` appended to `parent`.
fn el(dom: &mut Dom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let id = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(id, k, v).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// A table:
///
/// ```text
/// <colgroup class=g> <col class=a> <col class=b span=2> </colgroup>
/// <colgroup> <col class=d> </colgroup>
/// <tbody>
///   <tr> c00 (rowspan 2)  c01 (colspan 2)        c03 </tr>
///   <tr>                  c11  c12               c13 </tr>
/// </tbody>
/// <tfoot> <tr> f0 (colspan 4) </tr> </tfoot>
/// ```
///
/// with the `<tfoot>` first in the tree (HTML processes it last).
fn table() -> (Dom, Vec<NodeId>) {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", &[]);
    let g = el(&mut dom, t, "colgroup", &[("class", "g")]);
    el(&mut dom, g, "col", &[("class", "a")]);
    el(&mut dom, g, "col", &[("class", "b"), ("span", "2")]);
    let g2 = el(&mut dom, t, "colgroup", &[]);
    el(&mut dom, g2, "col", &[("class", "d")]);
    let foot = el(&mut dom, t, "tfoot", &[]);
    let fr = el(&mut dom, foot, "tr", &[]);
    let f0 = el(&mut dom, fr, "td", &[("id", "f0"), ("colspan", "4")]);
    let body = el(&mut dom, t, "tbody", &[]);
    let r0 = el(&mut dom, body, "tr", &[]);
    let c00 = el(&mut dom, r0, "td", &[("id", "c00"), ("rowspan", "2")]);
    let c01 = el(&mut dom, r0, "th", &[("id", "c01"), ("colspan", "2")]);
    let c03 = el(&mut dom, r0, "td", &[("id", "c03")]);
    let r1 = el(&mut dom, body, "tr", &[]);
    let c11 = el(&mut dom, r1, "td", &[("id", "c11")]);
    let c12 = el(&mut dom, r1, "td", &[("id", "c12")]);
    let c13 = el(&mut dom, r1, "td", &[("id", "c13")]);
    (dom, vec![c00, c01, c03, c11, c12, c13, f0])
}

/// The ids of the cells `selector` matches, in tree order.
fn ids(dom: &Dom, selector: &str) -> Vec<String> {
    dom.query_selector_all(selector)
        .iter()
        .map(|n| n.get_attribute("id").unwrap_or("?").to_string())
        .collect()
}

// ── Parsing ────────────────────────────────────────────────────────

/// Selectors 4 §16.1: `||` is a combinator, white space around it
/// optional; §16.2 / §16.3: `:nth-col(An+B)` / `:nth-last-col(An+B)`, no
/// `of S`.
#[test]
fn the_column_selectors_parse() {
    for text in ["col.a || td", "col.a||td"] {
        let list = selectors::parse(text).unwrap();
        assert_eq!(list.0[0].ancestors[0].0, Combinator::Column, "{text}");
    }
    let list = selectors::parse("td:nth-col(2n+1)").unwrap();
    assert!(matches!(
        list.0[0].subject.simples[1],
        SimpleSelector::NthColumn(ref n) if !n.last && n.a == 2 && n.b == 1
    ));
    let list = selectors::parse(":nth-last-col(odd)").unwrap();
    assert!(matches!(
        list.0[0].subject.simples[0],
        SimpleSelector::NthColumn(ref n) if n.last
    ));
    for bad in [
        ":nth-col(2 of td)",
        ":nth-col()",
        "td || ",
        "|| td",
        "a | | b",
    ] {
        assert!(selectors::parse(bad).is_err(), "{bad}");
    }
}

/// Selectors 4 §17: `:nth-col()` is a pseudo-class; the combinator
/// counts nothing.
#[test]
fn the_column_selectors_specificity() {
    let s = |t: &str| selectors::parse(t).unwrap().0[0].specificity();
    assert_eq!(s("col.a || td"), (0, 1, 2));
    assert_eq!(s("td:nth-col(1)"), (0, 1, 1));
    assert_eq!(s(":nth-last-col(1)"), (0, 1, 0));
}

// ── Matching ───────────────────────────────────────────────────────

/// Selectors 4 §16.1: `col || td` matches the cells of the columns the
/// column element represents — a `<col span>` its span, a `<colgroup>`
/// its columns — a cell spanning several columns belonging to each, by
/// HTML's table model (`rowspan` pushing the next row's cells aside, the
/// `<tfoot>` processed last but its cells in the same columns).
#[test]
fn the_column_combinator_matches_the_cells_of_a_column() {
    let (dom, _) = table();
    assert_eq!(ids(&dom, "col.a || td"), ["f0", "c00"]);
    assert_eq!(
        ids(&dom, "col.b || td, col.b || th"),
        ["f0", "c01", "c11", "c12"]
    );
    assert_eq!(ids(&dom, "col.d || *"), ["f0", "c03", "c13"]);
    assert_eq!(ids(&dom, ".g || td"), ["f0", "c00", "c11", "c12"]);
    assert_eq!(ids(&dom, "col.a || th"), Vec::<String>::new());
}

/// Selectors 4 §16.2 / §16.3: `:nth-col(An+B)` matches a cell with
/// `An+B - 1` columns before one of its columns; `:nth-last-col()` counts
/// from the table's last column.
#[test]
fn nth_col_counts_columns() {
    let (dom, _) = table();
    assert_eq!(ids(&dom, ":nth-col(1)"), ["f0", "c00"]);
    assert_eq!(ids(&dom, ":nth-col(3)"), ["f0", "c01", "c12"]);
    assert_eq!(ids(&dom, "td:nth-col(even)"), ["f0", "c03", "c11", "c13"]);
    assert_eq!(ids(&dom, ":nth-last-col(1)"), ["f0", "c03", "c13"]);
    assert_eq!(ids(&dom, ":nth-last-col(4)"), ["f0", "c00"]);
}

/// Selectors 4 §16.1: column membership is the document language's —
/// HTML's table model — so an element that is no cell of an HTML table
/// (a `<td>` outside one, a `<div>` whatever its CSS `display`) belongs to
/// no column.
#[test]
fn only_html_table_cells_belong_to_columns() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let loose = el(&mut dom, root, "td", &[("id", "loose")]);
    let div = el(&mut dom, root, "div", &[("id", "div")]);
    el(&mut dom, div, "col", &[]);
    let _ = loose;
    assert_eq!(ids(&dom, ":nth-col(1)"), Vec::<String>::new());
    assert_eq!(ids(&dom, "col || *"), Vec::<String>::new());
}

/// One table model a pass: matching every cell of a long table reads it
/// from the pass's caches.
#[test]
fn a_pass_models_each_table_once() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", &[]);
    let g = el(&mut dom, t, "colgroup", &[]);
    el(&mut dom, g, "col", &[("class", "x")]);
    let mut cells = Vec::new();
    for _ in 0..50 {
        let tr = el(&mut dom, t, "tr", &[]);
        cells.push(el(&mut dom, tr, "td", &[]));
        cells.push(el(&mut dom, tr, "td", &[]));
    }
    let list = selectors::parse("col.x || td, td:nth-last-col(2)").unwrap();
    let mut caches = SelectorCaches::new();
    let hits = cells
        .iter()
        .filter(|&&c| dom.matches_list_with(c, &list, None, &mut caches))
        .count();
    assert_eq!(hits, 50);
    assert_eq!(caches.work().table_models, 1);
}
