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

/// Selectors 4 §16.1 with §3.1: a cell spanning two column groups belongs
/// to the columns of both, and the combinator holds when *some* column
/// element satisfies the rest of the selector — a first candidate failing
/// on its ancestors (its `<colgroup>` lacks `.hl`) leaves the next to try.
#[test]
fn a_cell_spanning_two_column_groups_matches_through_either() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", &[]);
    let g0 = el(&mut dom, t, "colgroup", &[]);
    el(&mut dom, g0, "col", &[]);
    let g1 = el(&mut dom, t, "colgroup", &[("class", "hl")]);
    el(&mut dom, g1, "col", &[]);
    let tr = el(&mut dom, t, "tr", &[]);
    el(&mut dom, tr, "td", &[("id", "wide"), ("colspan", "2")]);
    let tr = el(&mut dom, t, "tr", &[]);
    el(&mut dom, tr, "td", &[("id", "left")]);
    el(&mut dom, tr, "td", &[("id", "right")]);
    assert_eq!(ids(&dom, ".hl col || td"), ["wide", "right"]);
    assert_eq!(ids(&dom, "table > .hl > col || td"), ["wide", "right"]);
}

/// HTML §13.2.6.4.9 ("in table": a `col` start tag inserts an implied
/// `<colgroup>`): a `<col>` that is a child of the `<table>`, before its
/// rows, is a column — the model reads the DOM rdom-parser builds, which
/// has no implied element, as an HTML parser's DOM would read.
#[test]
fn a_bare_col_before_the_rows_is_a_column() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", &[]);
    el(&mut dom, t, "col", &[("class", "p")]);
    el(&mut dom, t, "col", &[("class", "q"), ("span", "2")]);
    let tr = el(&mut dom, t, "tr", &[]);
    el(&mut dom, tr, "td", &[("id", "a")]);
    el(&mut dom, tr, "td", &[("id", "b")]);
    el(&mut dom, tr, "td", &[("id", "c")]);
    el(&mut dom, t, "col", &[("class", "late")]);
    assert_eq!(ids(&dom, "col.p || td"), ["a"]);
    assert_eq!(ids(&dom, "col.q || td"), ["b", "c"]);
    // HTML forms columns from the column groups before the first row only.
    assert_eq!(ids(&dom, "col.late || td"), Vec::<String>::new());
}

/// HTML's table elements are recognised by name ASCII case-insensitively
/// (`TD` is a `td`), for the column model as for layout — though type
/// *selectors* stay case-sensitive in rdom (DIVERGENCES), so the test
/// selects by class.
#[test]
fn upper_case_table_elements_are_table_elements() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "TABLE", &[]);
    let g = el(&mut dom, t, "COLGROUP", &[]);
    el(&mut dom, g, "COL", &[("class", "x"), ("span", "2")]);
    let tr = el(&mut dom, t, "TR", &[]);
    el(&mut dom, tr, "TD", &[("id", "a"), ("colspan", "2")]);
    el(&mut dom, tr, "Td", &[("id", "b")]);
    assert_eq!(ids(&dom, ".x || *"), ["a"]);
    assert_eq!(ids(&dom, ":nth-col(3)"), ["b"]);
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

// ── The column cap (C14G-CORE-GAPS) ──────────────────────────────────

/// Architect N16: the column model stops at [`crate::table::MAX_COLUMNS`]
/// (65 535) as the layout's grid does — a cell starting past it has no
/// column (layout gives it no box), so `:nth-col()` does not match it and
/// a `<col>` past it represents nothing (they matched at column 69 001).
#[test]
fn the_column_model_stops_where_layout_does() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let t = el(&mut dom, root, "table", &[]);
    let g = el(&mut dom, t, "colgroup", &[("span", "1000")]);
    for _ in 1..70 {
        el(&mut dom, t, "colgroup", &[("span", "1000")]);
    }
    let _ = g;
    let far = el(&mut dom, t, "col", &[("class", "far")]);
    let _ = far;
    let r = el(&mut dom, t, "tr", &[]);
    for i in 0..70 {
        el(
            &mut dom,
            r,
            "td",
            &[("id", &format!("c{i}")), ("colspan", "1000")],
        );
    }
    assert_eq!(ids(&dom, "td:nth-col(65000)"), ["c64"]);
    assert!(ids(&dom, "td:nth-col(69001)").is_empty());
    assert!(ids(&dom, "col.far || td").is_empty());
    assert_eq!(ids(&dom, "td:nth-col(65535)"), ["c65"], "cut at the cap");
}
