//! HTML tables on the table formatting context (C13-TFC part 4): the UA
//! sheet gives `<table>` and its parts their `display` values (HTML
//! §15.3.8), as browsers' do.

use rdom_tui::{CascadeExt, Display, Flow, LayoutRect, TablePart, TuiNodeExt};

use super::{by_id, doc, paint, rows};

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

/// HTML §15.3.8: `table { display: table }`, `caption { display:
/// table-caption }`, `colgroup` / `col` / `thead` / `tbody` / `tfoot` /
/// `tr` / `td, th` their parts.
#[test]
fn the_ua_sheet_gives_table_elements_their_display() {
    let mut dom = doc(
        r#"<table id="t"><caption id="cap">c</caption><colgroup id="cg"><col id="col"></colgroup><thead id="th"><tr><th id="h">h</th></tr></thead><tbody id="tb"><tr id="tr"><td id="td">d</td></tr></tbody><tfoot id="tf"></tfoot></table>"#,
    );
    dom.cascade(&rdom_tui::Stylesheet::new());
    let d = |id: &str| {
        let c = dom.node(by_id(&dom, id)).computed().cloned().unwrap();
        (c.display, c.flow)
    };
    use TablePart::*;
    assert_eq!(d("t"), (Display::Block, Flow::Table));
    for (id, part) in [
        ("cap", Caption),
        ("cg", ColumnGroup),
        ("col", Column),
        ("th", HeaderGroup),
        ("tb", RowGroup),
        ("tf", FooterGroup),
        ("tr", Row),
        ("h", Cell),
        ("td", Cell),
    ] {
        assert_eq!(d(id).0, Display::TablePart(part), "{id}");
    }
}

/// HTML §4.9.12.1 / CSS 2.1 §17.2: a `<tfoot>` before the `<tbody>`
/// renders last, `rowspan` spans rows (the flex-row tables ignored it),
/// and the cells line up in columns without a pre-pass.
#[test]
fn html_tables_lay_out_in_the_table_formatting_context() {
    let mut dom = doc(
        r#"<div><table><tfoot><tr><td>F</td><td>f</td></tr></tfoot><tbody><tr><td id="r" rowspan="2">R</td><td>x</td></tr><tr><td>yy</td></tr></tbody></table></div>"#,
    );
    let buf = paint(&mut dom, "", 20, 4);
    assert_eq!(rows(&buf)[..3], [" R  x", "    yy", " F  f"]);
    assert_eq!(rect(&dom, "r").height, 2);
}

/// CSS 2.1 §17.5.3: a row is as tall as its tallest cell — the UA sheet
/// no longer pins `<tr>` to one row.
#[test]
fn a_wrapped_cell_makes_its_row_taller() {
    let mut dom = doc(
        r#"<div class="w"><table><tr><td>aaa bbb</td><td>c</td></tr><tr><td>d</td><td id="e">e</td></tr></table></div>"#,
    );
    paint(&mut dom, ".w { width: 9 }", 20, 4);
    assert_eq!(rect(&dom, "e").y, 2);
}

/// CSS 2.1 §17.3 / §17.5.2.2: a `<col>`'s `width` sizes its column, and a
/// collapsed `<col>` removes it (§17.5.5).
#[test]
fn col_elements_size_and_collapse_their_columns() {
    let mut dom = doc(
        r#"<div><table><colgroup><col class="w"><col class="x"><col></colgroup><tr><td id="a">a</td><td>b</td><td id="c">c</td></tr></table></div>"#,
    );
    let buf = paint(
        &mut dom,
        ".w { width: 6 } .x { visibility: collapse }",
        20,
        2,
    );
    assert_eq!(rect(&dom, "a").width, 6);
    assert_eq!(rect(&dom, "c").x, 6);
    assert_eq!(rows(&buf)[0], " a     c");
}

/// CSS 2.1 §17.6.1: `border-spacing` on an HTML table spaces its cells.
#[test]
fn border_spacing_spaces_html_table_cells() {
    let mut dom = doc(r#"<div><table><tr><td id="a">a</td><td id="b">b</td></tr></table></div>"#);
    paint(&mut dom, "table { border-spacing: 2 1 }", 20, 4);
    assert_eq!((rect(&dom, "a").x, rect(&dom, "a").y), (2, 1));
    assert_eq!(rect(&dom, "b").x, 7);
}

/// HTML §15.3.8: "a rule … that matches `th` elements that have a parent
/// node whose computed value for the `text-align` property is its initial
/// value, whose declaration block consists of just a single declaration
/// that sets the `text-align` property to the value `center`" — a header
/// cell is centred unless its row inherited an alignment, and an author's
/// `th` rule (any value, `inherit` included) wins.
#[test]
fn th_is_centred_unless_its_parent_aligns_text() {
    use rdom_tui::TextAlign;
    let all = |css: &str| {
        let mut dom = doc(r#"<table><tr id="r"><th id="h">h</th></tr></table>"#);
        let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
        dom.cascade(&sheet);
        let c = dom.node(by_id(&dom, "h")).computed().cloned().unwrap();
        c.text.text_align_all
    };
    assert_eq!(all(""), TextAlign::Center);
    assert_eq!(all("table { text-align: right }"), TextAlign::Right);
    assert_eq!(all("table { text-align: end }"), TextAlign::End);
    // `start` is the initial value: still centred.
    assert_eq!(all("tr { text-align: start }"), TextAlign::Center);
    assert_eq!(all("th { text-align: left }"), TextAlign::Left);
    assert_eq!(all("th { text-align: inherit }"), TextAlign::Start);
}

/// HTML §15.3.8: `th` centred over its column (and bold), `td` at the start.
#[test]
fn header_cells_paint_centred_over_their_columns() {
    let mut dom = doc(
        r#"<div><table><tr><th>Name</th><th>Size</th></tr><tr><td>alphabet</td><td>1</td></tr></table></div>"#,
    );
    let buf = paint(&mut dom, "", 20, 3);
    assert_eq!(rows(&buf)[..2], ["   Name    Size", " alphabet  1"]);
    let right = paint(&mut dom, "table { text-align: right }", 20, 3);
    assert_eq!(rows(&right)[..2], ["     Name  Size", " alphabet     1"]);
}

/// HTML §15.3.8: `caption { text-align: center }` — and nothing more: no
/// italic, no colour of its own.
#[test]
fn a_caption_is_centred_and_plain() {
    let mut dom = doc(
        r#"<div><table id="t"><caption id="c">Cap</caption><tr><td>abcdefghi</td></tr></table></div>"#,
    );
    let buf = paint(&mut dom, "", 20, 3);
    assert_eq!(rows(&buf)[..2], ["    Cap", " abcdefghi"]);
    let c = dom.node(by_id(&dom, "c")).computed().cloned().unwrap();
    let t = dom.node(by_id(&dom, "t")).computed().cloned().unwrap();
    assert!(!c.modifiers.contains(rdom_tui::Modifier::ITALIC));
    assert_eq!(c.fg, t.fg);
}
