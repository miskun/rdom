//! The table properties (C13-TABLE-PROPS; CSS 2.1 §17.5.3, §17.6.1.1):
//! `vertical-align` on cells and `empty-cells`.

use super::{doc, paint, rows};

const PARTS: &str = ".t { display: table } .r { display: table-row } \
    .c { display: table-cell }";

fn css(extra: &str) -> String {
    format!("{PARTS} {extra}")
}

/// CSS 2.1 §17.5.3: a cell's content sits at the top of its row
/// (`top`), centred in it (`middle`, the upper of two rows when the free
/// space is odd) or at its bottom (`bottom`).
#[test]
fn top_middle_and_bottom_cells_place_their_content() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="c tall top">T</div><div class="c top">a</div><div class="c mid">m</div><div class="c bot">b</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(
            ".tall { height: 3 } .top { vertical-align: top } .mid { vertical-align: middle } .bot { vertical-align: bottom }",
        ),
        10,
        4,
    );
    assert_eq!(rows(&buf)[..3], ["Ta", "  m", "   b"]);
}

/// §17.5.3: `baseline` cells share their row's baseline — the lowest of
/// their first lines' — and the row grows to hold them; any other
/// `vertical-align` value (`sub`, a length) is `baseline` on a cell.
#[test]
fn baseline_cells_share_the_rows_baseline() {
    let mut dom = doc(
        r#"<div><div class="t"><div class="r"><div class="c pad">A</div><div class="c">B<br>b</div><div class="c sub">C</div></div><div class="r"><div class="c">z</div></div></div></div>"#,
    );
    let buf = paint(
        &mut dom,
        &css(".pad { padding-top: 2 } .sub { vertical-align: sub }"),
        10,
        6,
    );
    assert_eq!(rows(&buf)[..5], ["", "", "ABC", " b", "z"]);
}

/// HTML §15.3.8: `tbody, thead, tfoot, tr { vertical-align: middle }` and
/// `td, th { vertical-align: inherit }` — an HTML cell in a taller row is
/// centred, as in a browser.
#[test]
fn html_cells_are_middle_aligned() {
    let mut dom = doc(r#"<div><table><tr><td class="h">x</td><td>y</td></tr></table></div>"#);
    let buf = paint(&mut dom, ".h { height: 3 }", 10, 4);
    assert_eq!(rows(&buf)[..2], ["", " x  y"]);
}

/// CSS 2.1 §17.6.1.1: in the separated model `empty-cells: hide` draws no
/// border or background around a cell with no content; `show` (initial)
/// does, and the collapsing model ignores it.
#[test]
fn empty_cells_hide_drops_an_empty_cells_border() {
    let markup = r#"<div><div class="t"><div class="r"><div class="c">a</div><div class="c"></div></div></div></div>"#;
    let mut hide = doc(markup);
    let buf = paint(
        &mut hide,
        &css(".c { border: solid; width: 1 } .t { empty-cells: hide }"),
        10,
        4,
    );
    assert_eq!(rows(&buf)[..3], ["┌─┐", "│a│", "└─┘"]);
    let mut show = doc(markup);
    let buf = paint(&mut show, &css(".c { border: solid; width: 1 }"), 10, 4);
    assert_eq!(rows(&buf)[..3], ["┌─┐┌─┐", "│a││ │", "└─┘└─┘"]);
}
