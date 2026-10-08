//! C13G-DOCS (API N5): the table patterns a data grid is built from —
//! zebra rows and a scrolling wrapper — pinned where they paint.

use rdom_tui::render::{Buffer, Color};
use rdom_tui::{LayoutRect, TuiAccessors, TuiNodeExt};

use super::{by_id, doc, paint, rows};

fn bg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).expect("in the buffer").bg
}

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

/// CSS 2.1 §17.5.1: a row's background paints in its row, under its
/// transparent cells — `tr:nth-child(even)` stripes every cell of the
/// even rows, padding included, and nothing past the table.
#[test]
fn zebra_rows_paint_across_their_cells() {
    let mut dom = doc(
        r#"<div><table><tr><td>a</td><td>bb</td></tr><tr><td>c</td><td>d</td></tr><tr><td>e</td><td>f</td></tr></table></div>"#,
    );
    let buf = paint(
        &mut dom,
        "tr:nth-child(even) { background: rgb(1, 2, 3) }",
        20,
        4,
    );
    let stripe = Color::Rgb(1, 2, 3);
    for x in 0..7 {
        assert_eq!(bg(&buf, x, 1), stripe, "x={x}");
        assert_ne!(bg(&buf, x, 0), stripe, "x={x}");
        assert_ne!(bg(&buf, x, 2), stripe, "x={x}");
    }
    assert_ne!(bg(&buf, 7, 1), stripe);
}

/// CSS 2.1 §17.5.2.2 / CSS Overflow 3 §2.2: a table in a block `overflow:
/// auto` wrapper shrinks to it and wraps its cells, as in a browser; with
/// `width: max-content` it keeps one-line rows and the wrapper scrolls.
#[test]
fn a_wide_table_scrolls_in_an_overflow_auto_wrapper() {
    let markup = r#"<div><div id="w" class="w"><table id="t"><tr><td>alpha beta</td><td>gamma delta</td></tr></table></div></div>"#;
    let mut dom = doc(markup);
    paint(&mut dom, ".w { width: 16; overflow: auto }", 30, 6);
    assert_eq!(rect(&dom, "t").width, 16);
    assert!(rect(&dom, "t").height > 1);

    let mut dom = doc(markup);
    let buf = paint(
        &mut dom,
        ".w { width: 16; overflow: auto } table { width: max-content }",
        30,
        6,
    );
    assert_eq!(rect(&dom, "t").width, 25);
    assert_eq!(dom.node(by_id(&dom, "w")).scroll_width(), Some(25));
    assert_eq!(rows(&buf)[0], " alpha beta  gam");
}
