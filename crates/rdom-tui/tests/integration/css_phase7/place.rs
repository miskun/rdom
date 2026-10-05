//! C7-GRID-PLACE — placing grid items (CSS Grid 2 §8): line-based
//! placement with integers, names and spans (§8.3), its conflicts
//! (§8.3.1), the implicit tracks it creates (§7.5), and the
//! auto-placement algorithm with `grid-auto-flow` (§8.5).

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid `.g` holding one `span.<class>` per entry (text `x`), laid
/// out with `css` at 20 × 10. Returns each item's `(x, y, width)`.
fn place(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids: Vec<NodeId> = classes
        .iter()
        .map(|c| {
            let s = el(&mut dom, g, "span", c);
            let t = dom.create_text_node("x");
            dom.append_child(s, t).unwrap();
            s
        })
        .collect();
    lay_out(&mut dom, css, 20, 10);
    ids.iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width)
        })
        .collect()
}

const THREE: &str = ".g { display: grid; grid-template-columns: 2 2 2 }";

/// §8.5 step 1 then step 4: an item with definite lines on both axes is
/// placed first; the auto-placed ones fill the cells left, in order.
#[test]
fn definite_items_are_placed_first() {
    let at = place(
        &format!("{THREE} .a {{ grid-column: 3; grid-row: 1 }}"),
        &["a", "", ""],
    );
    assert_eq!(at, [(4, 0, 2), (0, 0, 2), (2, 0, 2)]);
}

/// §8.3: a negative line counts from the explicit grid's end — `-1` is
/// its last line — and `1 / -1` spans every explicit column.
#[test]
fn negative_lines_count_from_the_end() {
    let at = place(
        &format!("{THREE} .a {{ grid-column: -2 }} .all {{ grid-column: 1 / -1 }}"),
        &["a", "all"],
    );
    assert_eq!(at, [(4, 0, 2), (0, 1, 6)]);
}

/// §8.3: `span <n>` spans n tracks; an item that does not fit the rest
/// of the row starts the next one (§8.5 step 4, sparse).
#[test]
fn spans_cover_several_tracks() {
    let at = place(
        &format!("{THREE} .s {{ grid-column: span 2 }}"),
        &["", "s", ""],
    );
    assert_eq!(at, [(0, 0, 2), (2, 0, 4), (0, 1, 2)]);
}

/// §7.5 / §7.6: a line past the explicit grid adds implicit tracks,
/// sized by `grid-auto-columns`.
#[test]
fn lines_past_the_explicit_grid_add_implicit_tracks() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2; grid-auto-columns: 3 } \
         .a { grid-column: 3 }",
        &["a"],
    );
    assert_eq!(at, [(5, 0, 3)]);
}

/// §7.5 / §7.6: a line before the explicit grid adds implicit tracks
/// before it — the last one before the explicit grid takes the pattern's
/// last size — and auto-placement starts at the implicit grid's first
/// column (§8.5 step 4).
#[test]
fn lines_before_the_explicit_grid_add_implicit_tracks_before_it() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2; grid-auto-columns: 1 4 } \
         .a { grid-column: -3 }",
        &["a", ""],
    );
    assert_eq!(at, [(0, 0, 4), (4, 0, 2)]);
}

/// §8.3: a `<custom-ident>` is the first line of that name, `<n>
/// <ident>` the nth, and `span <ident>` reaches the next line of that
/// name from the other edge.
#[test]
fn named_lines_place_items() {
    let at = place(
        ".g { display: grid; grid-template-columns: [a] 2 [b] 3 [b c] 4 } \
         .first { grid-column: b } .second { grid-column: 2 b } \
         .to-c { grid-column: a / span c }",
        &["first", "second", "to-c"],
    );
    assert_eq!(at, [(2, 0, 3), (5, 0, 4), (0, 1, 5)]);
}

/// §8.3.1 (Grid 2 §8.3): a `<custom-ident>` alone first matches the line
/// named `<ident>-start` (`-end` for an end) — a named area's edge, the
/// hook `grid-template-areas` fills (C7-GRID-AREAS) — before a line
/// named `<ident>` itself.
#[test]
fn a_custom_ident_first_matches_its_start_and_end_lines() {
    let at = place(
        ".g { display: grid; \
         grid-template-columns: [main] 2 [main-start] 3 [main-end] 4 } \
         .m { grid-column: main }",
        &["m"],
    );
    assert_eq!(at, [(2, 0, 3)]);
}

/// §8.3: when there are fewer lines of a name than asked for, every
/// implicit line is taken to have it.
#[test]
fn missing_named_lines_are_implicit_lines() {
    let at = place(
        ".g { display: grid; grid-template-columns: [a] 2 2; grid-auto-columns: 3 } \
         .n { grid-column: 2 a }",
        &["n"],
    );
    // The second `a` is line 4, the first implicit line after the
    // explicit grid's 3.
    assert_eq!(at, [(7, 0, 3)]);
}

/// §8.3.1: a start line after its end line swaps with it; an end line
/// equal to the start drops; of two spans the end's drops; a lone span
/// to a name is a span of one.
#[test]
fn placement_conflicts_resolve() {
    let at = place(
        &format!(
            "{THREE} .swap {{ grid-column: 3 / 1 }} .same {{ grid-column: 2 / 2 }} \
             .spans {{ grid-column: span 2 / span 3 }} .named {{ grid-column: span z }}"
        ),
        &["swap", "same", "spans", "named"],
    );
    assert_eq!(at, [(0, 0, 4), (2, 1, 2), (0, 2, 4), (4, 2, 2)]);
}

/// §8.5 step 2: an item locked to a row takes that row's first free
/// column; the others auto-place from the start.
#[test]
fn an_item_locked_to_a_row_is_placed_before_the_others() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2 2 } .a { grid-row: 2 }",
        &["a", "", ""],
    );
    assert_eq!(at, [(0, 1, 2), (0, 0, 2), (2, 0, 2)]);
}

/// §8.5 step 4, sparse: an item with a definite column moves the cursor
/// to it; the next auto item continues after the cursor, not in an
/// earlier hole.
#[test]
fn a_definite_column_moves_the_cursor() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2 2 } .a { grid-column: 2 }",
        &["a", "", ""],
    );
    assert_eq!(at, [(2, 0, 2), (0, 1, 2), (2, 1, 2)]);
}

/// §7.7 / §8.5: `grid-auto-flow: column` fills each column before the
/// next; `dense` backfills the earliest hole that fits.
#[test]
fn grid_auto_flow_picks_the_direction_and_the_packing() {
    let at = place(
        ".g { display: grid; grid-auto-flow: column; grid-template-rows: 1 1; \
         grid-template-columns: 2 2 }",
        &["", "", ""],
    );
    assert_eq!(at, [(0, 0, 2), (0, 1, 2), (2, 0, 2)]);
    let css = |flow: &str| {
        format!("{THREE} .g {{ grid-auto-flow: {flow} }} .w {{ grid-column: span 2 }}")
    };
    // Sparse: the third item follows the second, in the second row.
    assert_eq!(
        place(&css("row"), &["w", "w", ""]),
        [(0, 0, 4), (0, 1, 4), (4, 1, 2)]
    );
    // Dense: it fills the hole left in the first row.
    assert_eq!(
        place(&css("dense"), &["w", "w", ""]),
        [(0, 0, 4), (0, 1, 4), (4, 0, 2)]
    );
}

/// §8.3 / §11.5: a spanning item's contribution is shared by the `auto`
/// tracks it spans — `abcdef` across two `auto` columns makes them 3
/// and 3 (§11.5.1), the single-span items above them having none wider.
#[test]
fn a_spanning_item_sizes_the_tracks_it_spans() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let mut item = |class: &str, text: &str| {
        let s = el(&mut dom, g, "span", class);
        let t = dom.create_text_node(text);
        dom.append_child(s, t).unwrap();
        s
    };
    let a = item("", "a");
    let b = item("", "b");
    let w = item("w", "abcdef");
    lay_out(
        &mut dom,
        ".g { display: grid; width: max-content; grid-template-columns: auto auto } \
         .w { grid-column: span 2 }",
        20,
        4,
    );
    assert_eq!((rect(&dom, a).width, rect(&dom, b).x), (3, 3));
    assert_eq!(rect(&dom, w).width, 6);
}

/// §8.5 step 2 runs before step 4: two items locked to the first row
/// take its cells first, side by side (sparse: each past the one before
/// it), and the earlier auto item moves to the next row.
#[test]
fn items_locked_to_a_row_take_it_before_auto_items() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2 2 } .a { grid-row: 1 }",
        &["", "a", "a"],
    );
    assert_eq!(at, [(0, 1, 2), (0, 0, 2), (2, 0, 2)]);
}

/// §8.5 step 4, sparse: a definite column behind the cursor's starts a
/// new row, though the current row has room there.
#[test]
fn a_definite_column_behind_the_cursor_starts_a_new_row() {
    let at = place(
        &format!("{THREE} .c {{ grid-column: 3 }} .a {{ grid-column: 1 }}"),
        &["c", "a"],
    );
    assert_eq!(at, [(4, 0, 2), (0, 1, 2)]);
}
