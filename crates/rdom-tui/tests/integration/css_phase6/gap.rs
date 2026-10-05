//! C6-GAP — `row-gap`, `column-gap` and the two-value `gap` (CSS Box
//! Alignment 3 §8) in flex layout; `normal` is 0 there (§8.1).

use super::{el, lay_out, rect};
use rdom_tui::TuiDom;

/// Three 2 × 1 items in a flex container `.f` under `css`; their x and y.
fn items(css: &str) -> Vec<(i32, i32)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..3).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        &format!(
            ".f {{ display: flex; width: 20; height: 10 }} .i {{ width: 2; height: 1 }} {css}"
        ),
        24,
        12,
    );
    ids.iter()
        .map(|&n| (rect(&dom, n).x, rect(&dom, n).y))
        .collect()
}

/// §8.1: in a row flex container the gutters between items are
/// `column-gap`; `row-gap` separates flex lines, of which a single-line
/// container has one.
#[test]
fn a_rows_items_are_spaced_by_column_gap() {
    let xs = |css: &str| items(css).into_iter().map(|(x, _)| x).collect::<Vec<_>>();
    assert_eq!(
        xs(".f { flex-direction: row; column-gap: 3; row-gap: 1 }"),
        [0, 5, 10]
    );
    assert_eq!(xs(".f { flex-direction: row; gap: 1 2 }"), [0, 4, 8]);
    assert_eq!(xs(".f { flex-direction: row; gap: normal }"), [0, 2, 4]);
}

/// §8.1: in a column flex container the gutters are `row-gap`.
#[test]
fn a_columns_items_are_spaced_by_row_gap() {
    let ys = |css: &str| items(css).into_iter().map(|(_, y)| y).collect::<Vec<_>>();
    assert_eq!(
        ys(".f { flex-direction: column; row-gap: 2; column-gap: 5 }"),
        [0, 3, 6]
    );
    assert_eq!(ys(".f { flex-direction: column; gap: 1 4 }"), [0, 2, 4]);
}
