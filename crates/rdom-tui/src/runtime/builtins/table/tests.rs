//! `<table>` column-width sync tests.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::table;
use crate::style::{Stylesheet, Value};

/// Build a table with two rows of two cells. Returns
/// (dom, cell_ids_row1, cell_ids_row2).
fn two_row_table(row1: [&str; 2], row2: [&str; 2]) -> (TuiDom, [NodeId; 2], [NodeId; 2]) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tbody = dom.create_element("tbody");

    let build_row = |dom: &mut TuiDom, cells: [&str; 2]| -> ([NodeId; 2], NodeId) {
        let tr = dom.create_element("tr");
        let td0 = dom.create_element("td");
        let t0 = dom.create_text_node(cells[0]);
        dom.append_child(td0, t0).unwrap();
        let td1 = dom.create_element("td");
        let t1 = dom.create_text_node(cells[1]);
        dom.append_child(td1, t1).unwrap();
        dom.append_child(tr, td0).unwrap();
        dom.append_child(tr, td1).unwrap();
        ([td0, td1], tr)
    };
    let (r1_cells, r1) = build_row(&mut dom, row1);
    let (r2_cells, r2) = build_row(&mut dom, row2);

    dom.append_child(tbody, r1).unwrap();
    dom.append_child(tbody, r2).unwrap();
    dom.append_child(table, tbody).unwrap();
    dom.append_child(root, table).unwrap();
    (dom, r1_cells, r2_cells)
}

fn cell_width(dom: &TuiDom, cell: NodeId) -> Option<u16> {
    dom.node(cell)
        .ext()
        .and_then(|e| match e.inline_style.width {
            Some(Value::Specified(Size::Fixed(w))) => Some(w),
            _ => None,
        })
}

/// Build an app from a pre-populated DOM. `App::build` runs the
/// table column-sync pass as part of its startup.
fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, Stylesheet::new(), terminal).unwrap()
}

// ── size_columns ──────────────────────────────────────────────────

#[test]
fn all_cells_in_the_same_column_get_equal_widths_after_sync() {
    // Column 0: "Alice" (5) vs "Bob" (3) → max 5 + 2 padding = 7.
    // Column 1: "30" (2) vs "25" (2) → max 2 + 2 padding = 4.
    let (mut dom, r1, r2) = two_row_table(["Alice", "30"], ["Bob", "25"]);
    let tables = {
        let mut out = Vec::new();
        for child in dom.node(dom.root()).child_nodes() {
            if child.tag_name() == Some("table") {
                out.push(child.id());
            }
        }
        out
    };
    for t in tables {
        table::size_columns(&mut dom, t);
    }
    assert_eq!(cell_width(&dom, r1[0]), Some(7));
    assert_eq!(cell_width(&dom, r2[0]), Some(7));
    assert_eq!(cell_width(&dom, r1[1]), Some(4));
    assert_eq!(cell_width(&dom, r2[1]), Some(4));
}

#[test]
fn app_build_auto_syncs_all_tables_in_the_tree() {
    let (dom, r1, r2) = two_row_table(["Alice", "Engineer"], ["Bob", "PM"]);
    let app = test_app(dom);
    // Column 0: "Alice" (5) / "Bob" (3) → 5 + 2 = 7
    // Column 1: "Engineer" (8) / "PM" (2) → 8 + 2 = 10
    assert_eq!(cell_width(app.dom(), r1[0]), Some(7));
    assert_eq!(cell_width(app.dom(), r2[0]), Some(7));
    assert_eq!(cell_width(app.dom(), r1[1]), Some(10));
    assert_eq!(cell_width(app.dom(), r2[1]), Some(10));
}

#[test]
fn empty_table_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    dom.append_child(root, table).unwrap();
    // No rows, no cells — size_columns should just return.
    table::size_columns(&mut dom, table);
}

#[test]
fn bare_tr_without_tbody_is_included() {
    // Same as two_row_table but skip the tbody wrapper.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tr1 = dom.create_element("tr");
    let td1 = dom.create_element("td");
    let t1 = dom.create_text_node("Wide content");
    dom.append_child(td1, t1).unwrap();
    dom.append_child(tr1, td1).unwrap();
    let tr2 = dom.create_element("tr");
    let td2 = dom.create_element("td");
    let t2 = dom.create_text_node("x");
    dom.append_child(td2, t2).unwrap();
    dom.append_child(tr2, td2).unwrap();
    dom.append_child(table, tr1).unwrap();
    dom.append_child(table, tr2).unwrap();
    dom.append_child(root, table).unwrap();

    table::size_columns(&mut dom, table);
    // "Wide content" (12) + 2 = 14.
    assert_eq!(cell_width(&dom, td1), Some(14));
    assert_eq!(cell_width(&dom, td2), Some(14));
}

#[test]
fn thead_and_tbody_rows_both_contribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let thead = dom.create_element("thead");
    let htr = dom.create_element("tr");
    let th = dom.create_element("th");
    let th_text = dom.create_text_node("Header");
    dom.append_child(th, th_text).unwrap();
    dom.append_child(htr, th).unwrap();
    dom.append_child(thead, htr).unwrap();
    let tbody = dom.create_element("tbody");
    let btr = dom.create_element("tr");
    let td = dom.create_element("td");
    let td_text = dom.create_text_node("X");
    dom.append_child(td, td_text).unwrap();
    dom.append_child(btr, td).unwrap();
    dom.append_child(tbody, btr).unwrap();
    dom.append_child(table, thead).unwrap();
    dom.append_child(table, tbody).unwrap();
    dom.append_child(root, table).unwrap();

    table::size_columns(&mut dom, table);
    // Max of "Header" (6) and "X" (1) → 6 + 2 = 8.
    assert_eq!(cell_width(&dom, th), Some(8));
    assert_eq!(cell_width(&dom, td), Some(8));
}

#[test]
fn nested_element_text_is_measured() {
    // A cell like <td><b>bold</b></td> should measure 4 chars,
    // not 0 — the pre-pass walks text descendants recursively.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    let b = dom.create_element("b");
    let t = dom.create_text_node("bold");
    dom.append_child(b, t).unwrap();
    dom.append_child(td, b).unwrap();
    dom.append_child(tr, td).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(root, table).unwrap();

    table::size_columns(&mut dom, table);
    // "bold" (4) + 2 padding = 6.
    assert_eq!(cell_width(&dom, td), Some(6));
}

#[test]
fn size_all_tables_handles_multiple_tables() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    // Two separate tables side by side under root.
    let tbl1 = {
        let t = dom.create_element("table");
        let tr = dom.create_element("tr");
        let td = dom.create_element("td");
        let text = dom.create_text_node("AAA");
        dom.append_child(td, text).unwrap();
        dom.append_child(tr, td).unwrap();
        dom.append_child(t, tr).unwrap();
        dom.append_child(root, t).unwrap();
        td
    };
    let tbl2 = {
        let t = dom.create_element("table");
        let tr = dom.create_element("tr");
        let td = dom.create_element("td");
        let text = dom.create_text_node("BBBBBB");
        dom.append_child(td, text).unwrap();
        dom.append_child(tr, td).unwrap();
        dom.append_child(t, tr).unwrap();
        dom.append_child(root, t).unwrap();
        td
    };

    table::size_all_tables(&mut dom);
    assert_eq!(cell_width(&dom, tbl1), Some(5)); // AAA=3+2
    assert_eq!(cell_width(&dom, tbl2), Some(8)); // BBBBBB=6+2
}

#[test]
fn cjk_graphemes_are_counted_as_wide() {
    // CJK ideographs are 2 display cells each. Two of them +
    // padding = 4 + 2 = 6.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    let t = dom.create_text_node("漢字");
    dom.append_child(td, t).unwrap();
    dom.append_child(tr, td).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(root, table).unwrap();

    table::size_columns(&mut dom, table);
    assert_eq!(cell_width(&dom, td), Some(6));
}
