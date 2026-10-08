//! HTML §15.3.8's table rules beyond `display` (C13G-TABLE-UA): `table {
//! box-sizing: border-box; text-indent: initial }`, `thead, tbody, tfoot,
//! table > tr { vertical-align: middle }` with `tr, td, th` inheriting it,
//! and `thead, tbody, tfoot, tr { border-color: inherit }`.

use rdom_tui::{CascadeExt, LayoutRect, TuiNodeExt, VerticalAlign};

use super::{by_id, doc, paint, rows};

fn rect(dom: &rdom_tui::TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

fn computed(dom: &rdom_tui::TuiDom, id: &str) -> rdom_tui::ComputedStyle {
    dom.node(by_id(dom, id)).computed().cloned().unwrap()
}

/// HTML §15.3.8 `table { box-sizing: border-box }`: a bordered `width:
/// 100%` table fits its container, in both border models (CSS 2.1
/// §17.6.1, §17.6.2).
#[test]
fn a_full_width_bordered_table_fits_its_container() {
    for model in ["separate", "collapse"] {
        let mut dom =
            doc(r#"<div class="w"><table id="t"><tr><td>a</td><td>b</td></tr></table></div>"#);
        let css = format!(
            ".w {{ width: 40 }} table {{ width: 100%; border: solid; border-collapse: {model} }}"
        );
        let buf = paint(&mut dom, &css, 50, 4);
        assert_eq!(rect(&dom, "t").width, 40, "{model}");
        let top = &rows(&buf)[0];
        assert_eq!(top.chars().count(), 40, "{model}: {top:?}");
    }
}

/// HTML §15.3.8: `tr` inherits `vertical-align` from its row group, so a
/// `tbody { vertical-align: top }` reaches its cells; a `<tr>` that is a
/// child of the `<table>` is `middle` itself.
#[test]
fn rows_inherit_vertical_align_from_their_group() {
    let mut dom = doc(
        r#"<div><table><tbody><tr id="r"><td class="tall">T</td><td id="c">c</td></tr></tbody></table><table><tr id="r2"><td id="c2">x</td></tr></table></div>"#,
    );
    let buf = paint(
        &mut dom,
        ".tall { height: 3 } tbody { vertical-align: top }",
        20,
        8,
    );
    assert_eq!(computed(&dom, "r").vertical_align, VerticalAlign::Top);
    assert_eq!(computed(&dom, "c").vertical_align, VerticalAlign::Top);
    assert_eq!(rows(&buf)[0], " T  c");
    assert_eq!(computed(&dom, "r2").vertical_align, VerticalAlign::Middle);
    assert_eq!(computed(&dom, "c2").vertical_align, VerticalAlign::Middle);
}

/// HTML §15.3.8 `table { text-indent: initial }`: an indented page does
/// not indent every cell.
#[test]
fn a_table_resets_text_indent() {
    let mut dom = doc(r#"<div class="p"><table><tr><td id="c">abc</td></tr></table></div>"#);
    let buf = paint(&mut dom, ".p { text-indent: 2 }", 20, 2);
    assert_eq!(rows(&buf)[0], " abc");
    assert_eq!(computed(&dom, "c").text.text_indent.resolve(10), 0);
}

/// HTML §15.3.8 `thead, tbody, tfoot, tr { border-color: inherit }`: a
/// row's borders take the table's colour.
#[test]
fn rows_and_groups_inherit_the_tables_border_color() {
    let mut dom =
        doc(r#"<table id="t"><thead id="h"><tr id="r"><td id="c">a</td></tr></thead></table>"#);
    let sheet = rdom_css::from_css_strict("table { border-color: red }").unwrap();
    dom.cascade(&sheet);
    let table = computed(&dom, "t").border_color;
    assert_eq!(computed(&dom, "h").border_color, table);
    assert_eq!(computed(&dom, "r").border_color, table);
    assert_ne!(computed(&dom, "c").border_color, table);
}
