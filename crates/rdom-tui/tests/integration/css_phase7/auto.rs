//! C7-GRID-AUTO — `grid-auto-columns` / `grid-auto-rows` (CSS Grid 2
//! §7.6): the sizes of the implicit tracks, repeated as a pattern.

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid `.g` holding `n` one-letter items, laid out with `css`.
fn grid(css: &str, n: usize, w: u16, h: u16) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids = (0..n)
        .map(|_| {
            let s = el(&mut dom, g, "span", "");
            let t = dom.create_text_node("x");
            dom.append_child(s, t).unwrap();
            s
        })
        .collect();
    lay_out(&mut dom, css, w, h);
    (dom, g, ids)
}

fn rows(dom: &TuiDom, ids: &[NodeId]) -> Vec<(i32, u16)> {
    ids.iter()
        .map(|&id| {
            let r = rect(dom, id);
            (r.y, r.height)
        })
        .collect()
}

/// CSS Grid 2 §7.6: the implicit rows auto-placement adds take
/// `grid-auto-rows`.
#[test]
fn implicit_rows_take_grid_auto_rows() {
    let (dom, g, ids) = grid(".g { display: grid; grid-auto-rows: 2 }", 3, 10, 8);
    assert_eq!(rows(&dom, &ids), [(0, 2), (2, 2), (4, 2)]);
    assert_eq!(rect(&dom, g).height, 6);
}

/// CSS Grid 2 §7.6: several sizes repeat as a pattern, "the first
/// implicit track after the last explicit track receives the first
/// specified size, and so on forwards".
#[test]
fn the_implicit_track_sizes_repeat_after_the_explicit_grid() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-rows: 1; grid-auto-rows: 2 3 }",
        4,
        10,
        10,
    );
    assert_eq!(rows(&dom, &ids), [(0, 1), (1, 2), (3, 3), (6, 2)]);
}

/// CSS Grid 2 §7.6 / §8.5 step 3: with no explicit column the implicit
/// grid has one, sized by `grid-auto-columns`.
#[test]
fn an_implicit_column_takes_grid_auto_columns() {
    let (dom, _, ids) = grid(".g { display: grid; grid-auto-columns: 5 }", 2, 10, 4);
    for &id in &ids {
        let r = rect(&dom, id);
        assert_eq!((r.x, r.width), (0, 5));
    }
}

/// CSS Grid 2 §7.6 / §11.7: an implicit `1fr` row is flexible like an
/// explicit one — three of them share a definite height of 6.
#[test]
fn flexible_implicit_rows_share_a_definite_height() {
    let (dom, _, ids) = grid(
        ".g { display: grid; height: 6; grid-auto-rows: 1fr }",
        3,
        10,
        8,
    );
    assert_eq!(rows(&dom, &ids), [(0, 2), (2, 2), (4, 2)]);
}
