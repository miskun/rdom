//! C7-SUBGRID — subgrids (CSS Grid 2 §9): a grid item whose
//! `grid-template-columns` / `-rows` is `subgrid` lays its items out in
//! the tracks its grid area spans in its parent — line names merged,
//! gaps inherited, its implicit grid clamped to that span — and its
//! items take part in sizing the parent's tracks, its margin, border
//! and padding with them (§9.5).

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// The tree `spec` describes — `(class, parent index, text)`, the parent
/// index into the list (`usize::MAX` for the root) — laid out with `css`
/// at 20 × 12. Returns each element's border box `(x, y, width,
/// height)`.
fn lay(css: &str, spec: &[(&str, usize, &str)]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut ids: Vec<NodeId> = Vec::new();
    for &(class, parent, text) in spec {
        let p = if parent == usize::MAX {
            root
        } else {
            ids[parent]
        };
        let id = el(&mut dom, p, "div", class);
        if !text.is_empty() {
            let t = dom.create_text_node(text);
            dom.append_child(id, t).unwrap();
        }
        ids.push(id);
    }
    lay_out(&mut dom, css, 20, 12);
    ids.iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

const ROOT: usize = usize::MAX;

/// §9: "the subgrid's own grid tracks … correspond to the parent grid's
/// tracks the subgrid spans": its items sit in the parent's columns.
#[test]
fn a_subgrids_items_sit_in_the_parents_tracks() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 3 5 4 } \
         .s { grid-column: 1 / 4; display: grid; grid-template-columns: subgrid }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("", 1, "a"),
            ("", 1, "b"),
            ("", 1, "c"),
        ],
    );
    assert_eq!(
        at[2..].iter().map(|r| (r.0, r.2)).collect::<Vec<_>>(),
        [(0, 3), (3, 5), (8, 4)]
    );
}

/// §9.5: a subgrid's items take part in sizing the parent's tracks they
/// span — its `aaaa` makes the parent's first `auto` column 4 wide, which
/// the parent's own item in the next row starts after.
#[test]
fn a_subgrids_items_size_the_parents_tracks() {
    let at = lay(
        ".g { display: grid; grid-template-columns: auto auto; justify-content: start } \
         .s { grid-column: 1 / 3; display: grid; grid-template-columns: subgrid } \
         .o { grid-column: 2 }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("", 1, "aaaa"),
            ("", 1, "b"),
            ("o", 0, "cc"),
        ],
    );
    assert_eq!((at[2].0, at[2].2), (0, 4));
    assert_eq!((at[3].0, at[3].2), (4, 2));
    assert_eq!(at[4].0, 4);
}

/// §9.5: "The subgrid's own border, padding, and margin … are applied
/// as an extra layer of (potentially negative) margin to the items at
/// those edges": `padding: 0 2` makes each edge column 2 wider than its
/// item, and the subgrid's content box takes the 2 off each edge track.
#[test]
fn a_subgrids_padding_counts_toward_its_edge_tracks() {
    let at = lay(
        ".g { display: grid; grid-template-columns: auto auto; justify-content: start } \
         .s { grid-column: 1 / 3; display: grid; grid-template-columns: subgrid; \
         padding: 0 2 } .o { grid-column: 2 }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("", 1, "aa"),
            ("", 1, "bb"),
            ("o", 0, "c"),
        ],
    );
    assert_eq!((at[1].0, at[1].2), (0, 8));
    assert_eq!((at[2].0, at[2].2), (2, 2));
    assert_eq!((at[3].0, at[3].2), (4, 2));
    assert_eq!(at[4].0, 4);
}

/// §9 "Line names": the subgrid's lines carry the parent's names for
/// them as well as its own `<line-name-list>`'s — `b` (its first line)
/// from the parent, `y` (its second) its own; a name it does not have
/// would be clamped into its last track instead.
#[test]
fn a_subgrids_line_names_merge_with_the_parents() {
    let at = lay(
        ".g { display: grid; grid-template-columns: [a] 2 [b] 3 [c] 4 } \
         .s { grid-column: 2 / 4; display: grid; grid-template-columns: subgrid [x] [y] } \
         .p { grid-column: b } .q { grid-column: y }",
        &[("g", ROOT, ""), ("s", 0, ""), ("p", 1, "p"), ("q", 1, "q")],
    );
    assert_eq!((at[2].0, at[2].2), (2, 3));
    assert_eq!((at[3].0, at[3].2), (5, 4));
}

/// §9 "Subgrid gutters": a subgrid's `normal` gap is its parent's; a gap
/// of its own takes half the difference off each side of the parent's
/// gutter.
#[test]
fn a_subgrid_inherits_or_narrows_the_parents_gap() {
    let css = |gap: &str| {
        format!(
            ".g {{ display: grid; grid-template-columns: 3 3; column-gap: 2 }} \
             .s {{ grid-column: 1 / 3; display: grid; grid-template-columns: subgrid; {gap} }}"
        )
    };
    let spec = [("g", ROOT, ""), ("s", 0, ""), ("", 1, "a"), ("", 1, "b")];
    let at = lay(&css(""), &spec);
    assert_eq!((at[2].0, at[2].2, at[3].0, at[3].2), (0, 3, 5, 3));
    let at = lay(&css("column-gap: 0"), &spec);
    assert_eq!((at[2].0, at[2].2, at[3].0, at[3].2), (0, 4, 4, 4));
}

/// §9: "the subgrid's implicit grid is clamped to its explicit grid" in
/// a subgridded axis — a line past the span lands in its last track,
/// and auto-placement wraps into new rows instead of new columns.
#[test]
fn a_subgrids_implicit_grid_is_clamped_to_its_span() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 3 5 4 } \
         .s { grid-column: 1 / 3; display: grid; grid-template-columns: subgrid } \
         .far { grid-column: 5; grid-row: 1 }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("far", 1, "f"),
            ("", 1, "a"),
            ("", 1, "b"),
        ],
    );
    assert_eq!((at[2].0, at[2].1, at[2].2), (3, 0, 5));
    assert_eq!((at[3].0, at[3].1), (0, 0));
    assert_eq!((at[4].0, at[4].1), (0, 1));
}

/// §9 on rows: a row subgrid's items sit in the parent's rows.
#[test]
fn a_row_subgrids_items_sit_in_the_parents_rows() {
    let at = lay(
        ".g { display: grid; grid-template-rows: 1 3 } \
         .s { grid-row: 1 / 3; display: grid; grid-template-rows: subgrid }",
        &[("g", ROOT, ""), ("s", 0, ""), ("", 1, "a"), ("", 1, "b")],
    );
    assert_eq!((at[2].1, at[2].3), (0, 1));
    assert_eq!((at[3].1, at[3].3), (1, 3));
}

/// §9: an auto-placed subgrid spans one track fewer than its
/// `<line-name-list>` names lines — three names, two columns.
#[test]
fn an_auto_placed_subgrid_spans_its_line_names() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 2 2 2 } \
         .s { display: grid; grid-template-columns: subgrid [a] [b] [c] }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("", 1, "a"),
            ("", 1, "b"),
            ("", 0, "o"),
        ],
    );
    assert_eq!((at[1].0, at[1].2), (0, 4));
    assert_eq!(at[4].0, 4);
}

/// §9: "If there is no parent grid … the used value is the initial
/// value, `none`" — a `subgrid` in block flow is an ordinary grid.
#[test]
fn subgrid_without_a_parent_grid_is_none() {
    let at = lay(
        ".s { display: grid; grid-template-columns: subgrid }",
        &[("s", ROOT, ""), ("", 0, "a"), ("", 0, "b")],
    );
    assert_eq!((at[1].1, at[2].1), (0, 1));
}

/// §9: a subgrid of a subgrid takes its tracks from its parent's, which
/// are the grandparent's — `.s2` spans the outer grid's last two
/// columns through `.s1`.
#[test]
fn nested_subgrids_reach_the_outer_tracks() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 3 5 4 } \
         .s1 { grid-column: 1 / 4; display: grid; grid-template-columns: subgrid } \
         .s2 { grid-column: 2 / 4; display: grid; grid-template-columns: subgrid }",
        &[
            ("g", ROOT, ""),
            ("s1", 0, ""),
            ("s2", 1, ""),
            ("", 2, "a"),
            ("", 2, "b"),
        ],
    );
    assert_eq!(
        at[3..].iter().map(|r| (r.0, r.2)).collect::<Vec<_>>(),
        [(3, 5), (8, 4)]
    );
}

/// §9: a subgrid's tracks are in its own writing mode — in an `rtl`
/// parent, an `rtl` subgrid's first column is the rightmost, an `ltr`
/// one's the leftmost of the parent's columns it spans.
#[test]
fn a_subgrid_orders_its_tracks_by_its_own_direction() {
    let at = |dir: &str| {
        lay(
            &format!(
                ".g {{ display: grid; direction: rtl; grid-template-columns: 3 5 4 }} \
                 .s {{ grid-column: 1 / 4; display: grid; grid-template-columns: subgrid; \
                 direction: {dir} }}"
            ),
            &[
                ("g", ROOT, ""),
                ("s", 0, ""),
                ("", 1, "a"),
                ("", 1, "b"),
                ("", 1, "c"),
            ],
        )[2..]
            .iter()
            .map(|r| (r.0, r.2))
            .collect::<Vec<_>>()
    };
    assert_eq!(at("rtl"), [(17, 3), (12, 5), (8, 4)]);
    assert_eq!(at("ltr"), [(8, 4), (12, 5), (17, 3)]);
}

/// C7G-MEMO-PURITY — §9 with §10.4: a baseline-aligned grid's subgrid
/// (stretched, so in no baseline group) takes its parent's two columns
/// from the first frame: `ab` and `cd` sit side by side and the subgrid
/// is one row tall. Measuring it for a baseline shim before its parent
/// had laid out its lines memoized it as one column, two rows.
#[test]
fn a_baseline_grids_subgrid_is_right_on_the_first_frame() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 3 3; align-items: baseline } \
         .s { display: grid; grid-column: 1 / 3; grid-template-columns: subgrid }",
        &[("g", ROOT, ""), ("s", 0, ""), ("", 1, "ab"), ("", 1, "cd")],
    );
    assert_eq!(at[1], (0, 0, 6, 1), "the subgrid");
    assert_eq!((at[2], at[3]), ((0, 0, 3, 1), (3, 0, 3, 1)), "its items");
}

/// C7G-SUBGRID-COST — §9.5: a subgrid's items size its parent's rows,
/// among them a subgrid of its own on the other axis, measured with the
/// columns it takes from it: `ab` and `cd` side by side in the outer
/// grid's two columns, so the shared row is one tall from the first
/// frame (measured without them, as one column, it was two).
#[test]
fn a_nested_subgrid_on_the_other_axis_sizes_its_ancestors_rows() {
    let at = lay(
        ".g { display: grid; grid-template-columns: 3 3 } \
         .s { display: grid; grid-column: 1 / 3; grid-template-rows: subgrid; \
         grid-template-columns: 3 3 } \
         .t { display: grid; grid-column: 1 / 3; grid-template-columns: subgrid }",
        &[
            ("g", ROOT, ""),
            ("s", 0, ""),
            ("t", 1, ""),
            ("", 2, "ab"),
            ("", 2, "cd"),
        ],
    );
    assert_eq!(at[0].3, 1, "the outer grid's row");
    assert_eq!((at[3], at[4]), ((0, 0, 3, 1), (3, 0, 3, 1)), "the items");
}
