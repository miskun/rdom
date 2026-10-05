//! C7-GRID-CORE — track lists and the track sizing algorithm (CSS Grid
//! 2 §7.2, §11): fixed, percentage, flexible and intrinsic tracks,
//! `minmax()`, `fit-content()`, `repeat()` with `auto-fill` / `auto-fit`,
//! and gutters. Free space is shared in whole cells (DIVERGENCES §1).

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid container `.g` under the root holding one `span` per entry of
/// `items` (its text; `""` for an empty item), laid out with `css` in a
/// `w` × `h` viewport. Returns the items.
fn grid(css: &str, items: &[&str], w: u16, h: u16) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids = items
        .iter()
        .map(|t| {
            let s = el(&mut dom, g, "span", "");
            if !t.is_empty() {
                let text = dom.create_text_node(t);
                dom.append_child(s, text).unwrap();
            }
            s
        })
        .collect();
    lay_out(&mut dom, css, w, h);
    (dom, g, ids)
}

/// `(x, width)` of each item.
fn columns(dom: &TuiDom, ids: &[NodeId]) -> Vec<(i32, u16)> {
    ids.iter()
        .map(|&id| {
            let r = rect(dom, id);
            (r.x, r.width)
        })
        .collect()
}

/// `(y, height)` of each item.
fn row_spans(dom: &TuiDom, ids: &[NodeId]) -> Vec<(i32, u16)> {
    ids.iter()
        .map(|&id| {
            let r = rect(dom, id);
            (r.y, r.height)
        })
        .collect()
}

/// CSS Grid 2 §7.2.4 / §11.7.1: an `fr` track takes its share of the
/// leftover space — what the non-flexible tracks leave — here 11 cells
/// over 3 fr, in whole cells rolled so the shares sum to the space
/// (3 + 8, not 3.67 + 7.33).
#[test]
fn fr_tracks_share_the_leftover_space() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: 2 1fr 2fr }",
        &["", "", ""],
        13,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 2), (2, 3), (5, 8)]);
}

/// CSS Grid 2 §10.1 / §11.1: gutters are treated as fixed tracks the
/// size of the gap; `row-gap` separates rows and `column-gap` columns.
#[test]
fn gaps_are_fixed_gutters() {
    let (dom, g, ids) = grid(
        ".g { display: grid; grid-template-columns: 2 1fr 2fr; gap: 1 2 }",
        &["a", "b", "c", "d"],
        13,
        4,
    );
    // 13 − 2 − 2 × 2 leaves 7 over 3 fr: 2 + 5.
    assert_eq!(columns(&dom, &ids[..3]), [(0, 2), (4, 2), (8, 5)]);
    assert_eq!(row_spans(&dom, &ids), [(0, 1), (0, 1), (0, 1), (2, 1)]);
    assert_eq!(rect(&dom, g).height, 3);
}

/// CSS Grid 2 §7.2.1: a percentage track is a percentage of the grid
/// container's content box on its axis.
#[test]
fn percentage_tracks_resolve_against_the_content_box() {
    let (dom, _, ids) = grid(
        ".g { display: grid; padding: 0 2; grid-template-columns: 50% 25% }",
        &["", ""],
        20,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(2, 8), (10, 4)]);
}

/// CSS Grid 2 §11.5 / §11.8: an `auto` track's base size is its items'
/// minimum contribution (here the min-content width), its growth limit
/// their max-content width; with `justify-content: normal` the auto
/// tracks then share the free space equally (7 each of 14).
#[test]
fn auto_tracks_fit_their_content_then_stretch() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: auto auto }",
        &["ab", "cdef"],
        20,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 9), (9, 11)]);
}

/// CSS Grid 2 §11.5 step 2: a `min-content` track is its items' largest
/// min-content contribution, a `max-content` track their max-content
/// one; the `1fr` track takes what is left (§11.7). An `auto` row is as
/// tall as its tallest item at its column's width — `aa bb` wraps to two
/// lines in its 2 cells, and every item in the row stretches to them.
#[test]
fn min_content_and_max_content_tracks() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: min-content max-content 1fr }",
        &["aa bb", "cc dd", "x"],
        20,
        3,
    );
    assert_eq!(columns(&dom, &ids), [(0, 2), (2, 5), (7, 13)]);
    assert_eq!(row_spans(&dom, &ids), [(0, 2), (0, 2), (0, 2)]);
}

/// CSS Grid 2 §7.2.3.1 / §11.4 / §11.6: `minmax(3, 6)` starts at 3 and
/// grows to 6 with the free space; `minmax(2, 1fr)` starts at 2 and, as
/// a flexible track, takes the leftover (§11.7).
#[test]
fn minmax_bounds_a_track() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: minmax(3, 6) minmax(2, 1fr) }",
        &["", ""],
        20,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 6), (6, 14)]);
}

/// CSS Grid 2 §7.2.2: `fit-content(4)` is `minmax(auto, max-content)`
/// with the growth limit clamped to 4 — `ab cd ef` (min-content 2,
/// max-content 8) gets 4 cells and wraps to three lines; `fit-content(10)`
/// over `abc` stays at its max-content 3.
#[test]
fn fit_content_clamps_the_growth_limit() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: fit-content(4) fit-content(10) 1fr }",
        &["ab cd ef", "abc", ""],
        30,
        4,
    );
    assert_eq!(columns(&dom, &ids), [(0, 4), (4, 3), (7, 23)]);
    assert_eq!(rect(&dom, ids[0]).height, 3);
}

/// CSS Grid 2 §7.2.3: `repeat(3, 2)` is three 2-cell tracks.
#[test]
fn repeat_expands_its_tracks() {
    let (dom, _, ids) = grid(
        ".g { display: grid; grid-template-columns: repeat(3, 2) 4 }",
        &["", "", "", ""],
        20,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 2), (2, 2), (4, 2), (6, 4)]);
}

/// CSS Grid 2 §7.2.3.2: `auto-fill` repeats as often as fits the content
/// box without overflowing, gaps included — 3 × 4 + 2 gaps in 15 — and
/// the fourth item starts the second row.
#[test]
fn auto_fill_repeats_as_many_tracks_as_fit() {
    let (dom, _, ids) = grid(
        ".g { display: grid; column-gap: 1; grid-template-columns: repeat(auto-fill, 4) }",
        &["a", "b", "c", "d"],
        15,
        3,
    );
    assert_eq!(columns(&dom, &ids), [(0, 4), (5, 4), (10, 4), (0, 4)]);
    assert_eq!(rect(&dom, ids[3]).y, 1);
}

/// CSS Grid 2 §7.2.3.2: the repetition count treats each track as its
/// max track sizing function when that is definite, else its minimum —
/// `minmax(4, 1fr)` counts as 4, so three tracks — which then flex:
/// 13 cells over 3 fr, rolled to 4 + 4 + 5.
#[test]
fn auto_fill_counts_a_flexible_track_at_its_minimum() {
    let (dom, _, ids) = grid(
        ".g { display: grid; column-gap: 1; \
         grid-template-columns: repeat(auto-fill, minmax(4, 1fr)) }",
        &["", "", ""],
        15,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 4), (5, 4), (10, 5)]);
}

/// CSS Grid 2 §7.2.3.2: `auto-fit` is `auto-fill` with the repeated
/// tracks that hold no item collapsed — sized 0, their gutters
/// collapsed — so the two items' flexible tracks share the whole row.
#[test]
fn auto_fit_collapses_the_empty_repetitions() {
    let (dom, _, ids) = grid(
        ".g { display: grid; column-gap: 1; \
         grid-template-columns: repeat(auto-fit, minmax(4, 1fr)) }",
        &["", ""],
        15,
        2,
    );
    assert_eq!(columns(&dom, &ids), [(0, 7), (8, 7)]);
}

/// CSS Grid 2 §11.7: with a definite height, flexible rows share what
/// the fixed row leaves — 5 cells over 3 fr, rolled to 1 + 4.
#[test]
fn flexible_rows_share_a_definite_height() {
    let (dom, _, ids) = grid(
        ".g { display: grid; height: 6; grid-template-rows: 1 1fr 2fr }",
        &["a", "b", "c"],
        10,
        8,
    );
    assert_eq!(row_spans(&dom, &ids), [(0, 1), (1, 1), (2, 4)]);
}

/// CSS Grid 2 §11.7, indefinite free space: in an `auto`-height grid the
/// flex fraction is the largest an item or a track's base size needs —
/// one row of text each in `1fr` and `2fr` makes the fraction 1, so the
/// rows are 1 and 2.
#[test]
fn flexible_rows_of_an_auto_height_grid_size_to_their_items() {
    let (dom, g, ids) = grid(
        ".g { display: grid; grid-template-rows: 1fr 2fr }",
        &["a", "b"],
        10,
        8,
    );
    assert_eq!(row_spans(&dom, &ids), [(0, 1), (1, 2)]);
    assert_eq!(rect(&dom, g).height, 3);
}
