//! C7-GRID-ALIGN — baseline self-alignment in grid (CSS Box Alignment 3
//! §9, CSS Grid 2 §10.4 and §11.5 step 1): the items of a row whose
//! `align-self` is `baseline` (`last baseline`) form a baseline-sharing
//! group and line their first (last) baselines up; the shims that takes
//! count toward the row's size.

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid `.g` holding one `div.<class>` per entry (text `ab`), laid out
/// with `css` at 20 × 12. Returns each item's border box `(x, y, width,
/// height)`.
fn place(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids: Vec<NodeId> = classes
        .iter()
        .map(|c| {
            let s = el(&mut dom, g, "div", c);
            let t = dom.create_text_node("ab");
            dom.append_child(s, t).unwrap();
            s
        })
        .collect();
    lay_out(&mut dom, css, 20, 12);
    ids.iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

/// Box Alignment §9.3 / §9.1: the items of a row aligned `baseline`
/// share a group whose first baselines (each item's first content row)
/// line up, the group flush with the row's start; an item aligned
/// otherwise (`.c`, `start`) keeps its place.
#[test]
fn first_baselines_line_up_in_a_row() {
    let at = place(
        ".g { display: grid; grid-template-columns: 4 4 4; align-items: baseline } \
         .a { padding-top: 2 } .c { align-self: start }",
        &["a", "b", "c", "d"],
    );
    assert_eq!(at, [(0, 0, 4, 3), (4, 2, 4, 1), (8, 0, 4, 1), (0, 3, 4, 1)]);
}

/// Grid §11.5 step 1: "Shim baseline-aligned items so their intrinsic
/// size contributions reflect their baseline alignment" — `.b` is
/// shifted down two rows to meet `.a`'s baseline, so the row holds its
/// four rows below that: six, and the next row starts there.
#[test]
fn the_shim_counts_toward_the_rows_size() {
    let at = place(
        ".g { display: grid; grid-template-columns: 4 4; align-items: baseline } \
         .a { padding-top: 2 } .b { padding-bottom: 3 }",
        &["a", "b", "c"],
    );
    assert_eq!(at, [(0, 0, 4, 3), (4, 2, 4, 4), (0, 6, 4, 1)]);
}

/// Box Alignment §9.1 / §9.3: `last baseline` lines up the last
/// baselines, the group flush with the row's end — `.a`'s last content
/// row (its third) and `.b`'s (its first, a padding row below it) meet
/// on row 3 of a 5-row track.
#[test]
fn last_baselines_line_up_at_the_rows_end() {
    let at = place(
        ".g { display: grid; grid-template-columns: 4 4; grid-template-rows: 5; \
         align-items: last baseline } .a { padding-top: 2 } .b { padding-bottom: 1 }",
        &["a", "b"],
    );
    assert_eq!(at, [(0, 1, 4, 3), (4, 3, 4, 2)]);
}

/// Box Alignment §9.3: an item spanning several rows joins the
/// first-baseline group of the first row it spans.
#[test]
fn a_spanning_item_aligns_in_its_first_row() {
    let at = place(
        ".g { display: grid; grid-template-columns: 4 4; align-items: baseline } \
         .a { grid-row: 1 / 3; padding-top: 2 }",
        &["a", "b"],
    );
    assert_eq!(at[1], (4, 2, 4, 1));
}
