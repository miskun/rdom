//! The column selectors in the cascade (C13-COLUMN; Selectors 4 §16):
//! `col || td` and `:nth-col()` style HTML table cells, and changing the
//! columns — a `span`, a `colspan`, a new cell — restyles them.

use rdom_tui::style::DirtyTracker;
use rdom_tui::{CascadeExt, Color, TuiNodeExt};

use super::{by_id, doc};

const RED: Color = Color::Rgb(255, 0, 0);

fn fg(dom: &rdom_tui::TuiDom, id: &str) -> Color {
    dom.node(by_id(dom, id)).computed().unwrap().fg
}

/// Selectors 4 §16.1 / §16.2: a column's rule styles its cells.
#[test]
fn column_selectors_style_cells() {
    let mut dom = doc(
        r#"<table><colgroup><col class="x"><col></colgroup><tr><td id="a">a</td><td id="b">b</td></tr></table>"#,
    );
    let sheet = rdom_css::from_css_strict(
        "col.x || td { color: rgb(255 0 0) } td:nth-col(2) { color: rgb(0 0 255) }",
    )
    .unwrap();
    dom.cascade(&sheet);
    assert_eq!(fg(&dom, "a"), RED);
    assert_eq!(fg(&dom, "b"), Color::Rgb(0, 0, 255));
}

/// A `span`, a `rowspan` or a `colspan` moves cells between columns: the
/// table's cells are restyled (the dirty tracker marks the table) — even
/// where the attribute is on another element than the cell.
#[test]
fn changing_the_columns_restyles_the_cells() {
    let mut dom = doc(
        r#"<table><colgroup><col id="first"><col class="x"></colgroup><tr><td id="a">a</td><td id="b">b</td></tr><tr><td id="c">c</td><td id="d">d</td></tr></table>"#,
    );
    let tracker = DirtyTracker::install(&mut dom);
    let sheet = rdom_css::from_css_strict("col.x || td { color: rgb(255 0 0) }").unwrap();
    dom.cascade(&sheet);
    let reds = |dom: &rdom_tui::TuiDom| ["a", "b", "c", "d"].map(|id| fg(dom, id) == RED);
    assert_eq!(reds(&dom), [false, true, false, true]);
    let change = |dom: &mut rdom_tui::TuiDom, id: &str, name: &str, value: &str| {
        let el = by_id(dom, id);
        dom.set_attribute(el, name, value).unwrap();
        let roots = tracker.take_roots();
        dom.cascade_subtrees(&sheet, &roots);
    };
    // The first `<col>` spanning two columns moves `col.x` to the third.
    change(&mut dom, "first", "span", "2");
    assert_eq!(reds(&dom), [false, false, false, false]);
    change(&mut dom, "first", "span", "1");
    assert_eq!(reds(&dom), [false, true, false, true]);
    // `a` spanning two rows pushes the second row's cells right.
    change(&mut dom, "a", "rowspan", "2");
    assert_eq!(reds(&dom), [false, true, true, false]);
    // And spanning two columns too, it is in the second.
    change(&mut dom, "a", "colspan", "2");
    assert_eq!(reds(&dom), [true, false, false, false]);
}
