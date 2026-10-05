//! C7G-DOCS-TESTS — `TuiAccessors::grid_tracks`: a laid-out grid's used
//! track sizes, the analogue of the resolved value of
//! `grid-template-columns` / `-rows` (CSS Grid 2 §7.2.6: "the resolved
//! value is the used value", every track listed, implicit ones
//! included), as cell ranges a consumer can draw headers or rules on.

use super::{el, lay_out};
use rdom_tui::{TuiAccessors, TuiDom};

/// Track ranges as `(start, end)` pairs.
type Ranges = Vec<(i32, i32)>;

/// The tracks of a grid `.g` holding `items` empty `div`s, laid out with
/// `css` at 20 × 10; `None` when `.g` is no grid.
fn tracks(css: &str, items: usize) -> Option<(Ranges, Ranges)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    for _ in 0..items {
        el(&mut dom, g, "div", "");
    }
    lay_out(&mut dom, css, 20, 10);
    let t = dom.node(g).grid_tracks()?;
    let pairs = |r: &[std::ops::Range<i32>]| r.iter().map(|r| (r.start, r.end)).collect();
    Some((pairs(t.columns()), pairs(t.rows())))
}

/// §7.2.6 and §11: each column and row as the cells it covers from the
/// content box's left / top edge — the gutters between them — with the
/// implicit row auto-placement adds (§7.6) after the explicit one, and
/// the container's padding outside the ranges.
#[test]
fn a_grids_tracks_are_cell_ranges_in_its_content_box() {
    let css = ".g { display: grid; padding: 1; width: 18; grid-template-columns: 4 1fr 3; \
               grid-template-rows: 2; grid-auto-rows: 1; column-gap: 1; row-gap: 2 }";
    let (columns, rows) = tracks(css, 4).expect("a grid");
    assert_eq!(columns, [(0, 4), (5, 14), (15, 18)]);
    assert_eq!(rows, [(0, 2), (4, 5)]);
}

/// CSS Writing Modes 4 §2.1 with CSS Grid 2 §7.1: an `rtl` grid's first
/// column is its rightmost; the ranges stay physical, left to right, in
/// grid order.
#[test]
fn an_rtl_grids_first_column_is_on_the_right() {
    let css = ".g { display: grid; direction: rtl; width: 10; grid-template-columns: 2 3 }";
    let (columns, _) = tracks(css, 0).expect("a grid");
    assert_eq!(columns, [(8, 10), (5, 8)]);
}

/// A box that is not a grid container has no tracks.
#[test]
fn a_block_has_no_grid_tracks() {
    assert_eq!(tracks(".g { width: 10 }", 1), None);
}
