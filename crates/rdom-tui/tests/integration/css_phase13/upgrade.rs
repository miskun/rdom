//! C13G-UPGRADE — the 0.5 → 0.6 table migration (`UPGRADING-0.6.md`,
//! `sc-tables` and "Porting a column-synced table"): each claim of it pinned, and the port of
//! a column-synced virtual table (rdom-virtualtable's shape) from 0.5's
//! `size_columns` / `table_used_width` / cell margins to CSS tables.

use rdom_tui::{LayoutRect, TuiAccessors, TuiAccessorsMut, TuiNodeExt};

use super::{by_id, doc, paint, rows};

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

fn widths(dom: &rdom_tui::TuiDom) -> Vec<usize> {
    let t = dom.node(by_id(dom, "t")).table_tracks().expect("a table");
    t.columns().iter().map(|c| c.len()).collect()
}

/// CSS Tables 3 "distributing excess width to columns": a `width: 100%`
/// table spreads the extra over its columns by their max-content widths —
/// it does not leave them content-wide and packed left, as 0.5 did.
#[test]
fn a_full_width_table_spreads_its_extra_width() {
    let mut dom =
        doc(r#"<div class="w"><table id="t"><tr><td>a</td><td>bbbb</td></tr></table></div>"#);
    paint(&mut dom, ".w { width: 20 } table { width: 100% }", 30, 2);
    let w = widths(&dom);
    assert_eq!(w.iter().sum::<usize>(), 20);
    assert!(w[0] > 3 && w[1] > 6, "{w:?}");
}

/// CSS 2.1 §17.5.3: cells wrap in a narrow table; `white-space: nowrap`
/// keeps a row one line, as 0.5's were.
#[test]
fn cells_wrap_unless_nowrap() {
    for (css, height) in [("", 2), ("td { white-space: nowrap }", 1)] {
        let mut dom = doc(
            r#"<div class="w"><table><tr><td id="c">aaa bbb</td><td>c</td></tr></table></div>"#,
        );
        paint(&mut dom, &format!(".w {{ width: 8 }} {css}"), 30, 4);
        assert_eq!(rect(&dom, "c").height, height, "{css:?}");
    }
}

/// §17.5.2.2: in the automatic algorithm a cell's `width` is a minimum —
/// wider content widens its column; `table-layout: fixed` with a table
/// width makes it exact (§17.5.2.1).
#[test]
fn a_cell_width_is_a_minimum_unless_fixed() {
    let markup =
        r#"<div><table id="t"><tr><td class="n">abcdefgh</td><td>x</td></tr></table></div>"#;
    let mut dom = doc(markup);
    paint(&mut dom, ".n { width: 3 }", 30, 2);
    assert_eq!(widths(&dom)[0], 10);
    let mut dom = doc(markup);
    paint(
        &mut dom,
        "table { table-layout: fixed; width: 8 } .n { width: 3 }",
        30,
        2,
    );
    assert_eq!(widths(&dom), [5, 3]);
}

/// CSS 2.1 §17.5: a table is no flex container and a cell has no margins,
/// so `td { flex: 1 }`, `gap` on the table and cell margins change
/// nothing.
#[test]
fn flex_gap_and_cell_margins_do_not_apply() {
    let markup = r#"<div><table id="t"><tr><td id="a">a</td><td id="b">b</td></tr></table></div>"#;
    let mut plain = doc(markup);
    paint(&mut plain, "", 30, 2);
    let mut styled = doc(markup);
    paint(
        &mut styled,
        "table { gap: 3 } td { flex: 1; margin: 0 4 }",
        30,
        2,
    );
    for id in ["a", "b"] {
        assert_eq!(rect(&styled, id), rect(&plain, id), "{id}");
    }
}

/// The port of a column-synced virtual table: 0.5 set a `width` on each
/// header cell, re-ran `size_columns` after every window change, read
/// `table_used_width` for `column_width()` and pinned an overflow chip
/// right with a header cell's `margin-left: auto`. In 0.6 the header's
/// widths are exact under `table-layout: fixed` with a table width (the
/// first row — the header — sizes the columns, so rows materializing
/// cannot move them), `column_width(i)` is `table_tracks()`'s column `i`,
/// and the chip leaves the table, positioned in the pane.
#[test]
fn a_virtual_table_ports_to_css_tables() {
    let mut dom = doc(
        r#"<div class="pane"><table id="t"><thead><tr><th class="name">Name</th><th class="size">Size</th><th class="when">Modified</th></tr></thead><tbody><tr><td>a-very-long-file-name</td><td>12</td><td>today</td></tr></tbody></table><span id="chip" class="chip">+3</span></div>"#,
    );
    let css = ".pane { position: relative; width: 30 } \
               table { table-layout: fixed; width: 100% } \
               .name { width: 10 } .size { width: 4 } .when { width: 10 } \
               td, th { white-space: nowrap; overflow: hidden } \
               .chip { position: absolute; top: 0; right: 0 }";
    let buf = paint(&mut dom, css, 40, 3);
    // `column_width(i)`: the border box of column `i` (padding included).
    assert_eq!(widths(&dom), [12, 6, 12]);
    // A long cell is clipped at its padding edge (CSS Overflow 3 §3),
    // not widening its column.
    assert_eq!(rows(&buf)[1], " a-very-long 12    today");
    // The chip sits at the pane's right edge, over the header row.
    assert_eq!((rect(&dom, "chip").x, rect(&dom, "chip").y), (28, 0));
    assert!(rows(&buf)[0].ends_with("+3"), "{:?}", rows(&buf)[0]);
}

/// C16G-TBODY-SCROLL (Phase 16 gate API B3). CSS 2.1 §11.1.1 and CSS
/// Overflow 3 §3: `overflow` applies to block containers, flex and grid
/// containers; a table row group is none of them, so a `<tbody>` given
/// `overflow-y: auto` is no scroll container — setting its `scrollTop`
/// does nothing (CSSOM View §4: only a scroll container scrolls), as in
/// browsers. rdom-virtualtable's 0.5 shape scrolled its `<tbody>`; the
/// port scrolls a wrapper instead (below).
#[test]
fn a_scrolling_tbody_does_not_scroll() {
    let mut dom = doc(
        r#"<div><table><tbody id="b"><tr><td>r1</td></tr><tr><td>r2</td></tr><tr><td>r3</td></tr><tr><td>r4</td></tr></tbody></table></div>"#,
    );
    let css = "tbody { overflow-y: auto; height: 2 }";
    paint(&mut dom, css, 20, 6);
    let b = by_id(&dom, "b");
    dom.node_mut(b).set_scroll_top(2).unwrap();
    let buf = paint(&mut dom, css, 20, 6);
    assert_eq!(
        dom.node(b).scroll_top(),
        Some(0),
        "a row group is no scroll container"
    );
    assert_eq!(rows(&buf)[0], " r1", "the rows unmoved");
}

/// The port's scrolling body (UPGRADING "Porting a column-synced table",
/// RECIPES "A table with a scrolling body"): a scrolling wrapper around
/// the table, its header cells `position: sticky; top: 0` (CSS Position
/// 3 §3.4: a sticky box sticks within its nearest scroll container, and
/// a cell's containing block is its table, CSS 2.1 §10.1) with a
/// background to cover the rows passing under it — a scroll moves the
/// body rows under a header that stays.
#[test]
fn a_table_scrolls_in_a_wrapper_under_a_sticky_header() {
    let mut dom = doc(
        r#"<div id="pane"><table><thead><tr><th>H</th></tr></thead><tbody><tr><td>r1</td></tr><tr><td>r2</td></tr><tr><td>r3</td></tr><tr><td>r4</td></tr><tr><td>r5</td></tr></tbody></table></div>"#,
    );
    let css = "#pane { overflow-y: auto; height: 3; scrollbar-width: none } \
               thead th { position: sticky; top: 0; background-color: Canvas } \
               th { text-align: left }";
    let buf = paint(&mut dom, css, 20, 4);
    assert_eq!(rows(&buf)[..3], [" H", " r1", " r2"]);
    let pane = by_id(&dom, "pane");
    dom.node_mut(pane).set_scroll_top(2).unwrap();
    let buf = paint(&mut dom, css, 20, 4);
    assert_eq!(dom.node(pane).scroll_top(), Some(2));
    assert_eq!(rows(&buf)[..3], [" H", " r3", " r4"], "the header stays");
}
