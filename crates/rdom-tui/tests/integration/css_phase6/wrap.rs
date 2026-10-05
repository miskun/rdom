//! C6-WRAP — `flex-wrap` and `flex-flow` (CSS Flexbox §5.2 / §5.3):
//! multi-line flex containers (§9.3 line breaking, §9.4 line cross
//! sizes, `align-content: normal` stretching the lines, §9.9 intrinsic
//! sizes), with `row-gap` / `column-gap` between the lines (CSS Box
//! Alignment 3 §8.1).

use super::{el, lay_out, rect, size};
use rdom_tui::{LayoutExt, TuiAccessors, TuiAccessorsMut, TuiDom};

/// Items `.i` with the given extra classes in a flex container `.f`
/// placed at the viewport's origin, laid out under `css`; the items'
/// rects as `(x, y, width, height)`.
fn items(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = classes
        .iter()
        .map(|c| {
            let id = el(&mut dom, f, "div", &format!("i {c}"));
            let t = dom.create_text_node("x");
            dom.append_child(id, t).unwrap();
            id
        })
        .collect();
    lay_out(&mut dom, &format!(".f {{ display: flex }} {css}"), 30, 20);
    ids.iter()
        .map(|&n| {
            let r = rect(&dom, n);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

fn xs(v: &[(i32, i32, u16, u16)]) -> Vec<i32> {
    v.iter().map(|r| r.0).collect()
}
fn ys(v: &[(i32, i32, u16, u16)]) -> Vec<i32> {
    v.iter().map(|r| r.1).collect()
}

/// §9.3 step 5: items are collected into a line until the next one's
/// outer hypothetical main size would not fit the inner main size —
/// `column-gap` counted between them (Box Alignment §8.1) — and the
/// lines stack along the cross axis `row-gap` apart. An `auto`-height
/// container is as tall as its lines (§9.4 step 8, §9.9).
#[test]
fn a_row_breaks_into_lines_that_stack() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..5).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        ".f { display: flex; flex-wrap: wrap; width: 10; column-gap: 1; row-gap: 1 } \
         .i { width: 4; height: 1 }",
        30,
        20,
    );
    let at: Vec<_> = ids
        .iter()
        .map(|&n| (rect(&dom, n).x, rect(&dom, n).y))
        .collect();
    assert_eq!(at, [(0, 0), (5, 0), (0, 2), (5, 2), (0, 4)]);
    assert_eq!(size(&dom, f), (10, 5));
}

/// §9.7 runs per line: each line's items share that line's free space.
#[test]
fn flexible_lengths_resolve_per_line() {
    let r = items(
        ".f { flex-wrap: wrap; width: 10 } .i { width: 4; flex-grow: 1 }",
        &["", "", ""],
    );
    assert_eq!(r.iter().map(|r| r.2).collect::<Vec<_>>(), [5, 5, 10]);
    assert_eq!(xs(&r), [0, 5, 0]);
    assert_eq!(ys(&r), [0, 0, 1]);
}

/// §9.4 steps 7–8 / 11: a multi-line container's line is as tall as its
/// tallest item's outer hypothetical cross size, and a `stretch` item
/// (`align-self: auto` → `normal`) fills its own line.
#[test]
fn a_line_is_as_tall_as_its_tallest_item() {
    let r = items(
        ".f { flex-wrap: wrap; width: 10 } .i { width: 4 } .tall { height: 3 }",
        &["tall", "", ""],
    );
    assert_eq!(ys(&r), [0, 0, 3]);
    assert_eq!(r.iter().map(|r| r.3).collect::<Vec<_>>(), [3, 3, 1]);
}

/// §9.4 step 15 with Box Alignment §5.3 / §6.1: `align-content: normal`
/// behaves as `stretch` in a flex container — the free cross space is
/// shared equally among the lines. Whole cells: a remainder cell goes
/// to the first lines.
#[test]
fn align_content_normal_stretches_the_lines() {
    let r = items(
        ".f { flex-wrap: wrap; width: 10; height: 9; row-gap: 1 } .i { width: 4 }",
        &["", "", ""],
    );
    assert_eq!(ys(&r), [0, 0, 5]);
    assert_eq!(r.iter().map(|r| r.3).collect::<Vec<_>>(), [4, 4, 4]);
    let r = items(
        ".f { flex-wrap: wrap; width: 10; height: 10; row-gap: 1 } .i { width: 4 }",
        &["", "", ""],
    );
    assert_eq!(ys(&r), [0, 0, 6]);
    assert_eq!(r.iter().map(|r| r.3).collect::<Vec<_>>(), [5, 5, 4]);
}

/// §5.2: `wrap-reverse` swaps cross-start and cross-end — the first line
/// sits at the cross-end edge (the bottom of a row) and the lines stack
/// toward cross-start. Under `rtl` each line's main axis also runs from
/// the right (CSS Writing Modes 4 §2.1), the two mirrors independent.
#[test]
fn wrap_reverse_stacks_lines_from_the_cross_end() {
    let five = ["", "", "", "", ""];
    let base = ".f { width: 10; height: 5; row-gap: 1; column-gap: 1; flex-wrap: wrap-reverse } \
                .i { width: 4; height: 1 }";
    let r = items(base, &five);
    assert_eq!(ys(&r), [4, 4, 2, 2, 0]);
    assert_eq!(xs(&r), [0, 5, 0, 5, 0]);
    let r = items(&format!("{base} .f {{ direction: rtl }}"), &five);
    assert_eq!(ys(&r), [4, 4, 2, 2, 0]);
    assert_eq!(xs(&r), [6, 1, 6, 1, 6]);
    // `row-reverse` with `wrap-reverse`: both axes from their ends.
    let r = items(
        &format!("{base} .f {{ flex-direction: row-reverse }}"),
        &five,
    );
    assert_eq!(xs(&r), [6, 1, 6, 1, 6]);
    assert_eq!(ys(&r), [4, 4, 2, 2, 0]);
}

/// §5.3: `flex-flow: column wrap` — a column breaks at its definite
/// height and its lines (columns) stack along the inline axis
/// `column-gap` apart.
#[test]
fn a_column_wraps_into_columns() {
    let r = items(
        ".f { flex-flow: column wrap; height: 4; width: 7; column-gap: 1 } \
         .i { height: 2; width: 3 }",
        &["", "", ""],
    );
    assert_eq!(xs(&r), [0, 0, 4]);
    assert_eq!(ys(&r), [0, 2, 0]);
}

/// §9.9.1: a multi-line row's min-content main size is its largest item
/// min-content contribution (each item can take a line of its own); its
/// max-content size is the items' sum, one line. Its `auto` height is
/// its lines' at that width (§9.9.2).
#[test]
fn intrinsic_sizes_of_a_wrapping_row() {
    for (width, expect) in [("min-content", (6, 3)), ("max-content", (14, 1))] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "div", "f");
        for c in ["i", "i w", "i"] {
            el(&mut dom, f, "div", c);
        }
        lay_out(
            &mut dom,
            &format!(
                ".f {{ display: flex; flex-wrap: wrap; width: {width} }} \
                 .i {{ width: 4; height: 1 }} .w {{ width: 6 }}"
            ),
            30,
            20,
        );
        assert_eq!(size(&dom, f), expect, "{width}");
    }
}

/// CSS Box 4 §3.2: in a multi-line flex container `margin-trim` trims
/// the main-axis margins of each line's first and last items and the
/// cross-axis margins of the items in the first and last lines.
#[test]
fn margin_trim_applies_per_line() {
    let r = items(
        ".f { flex-wrap: wrap; width: 12; margin-trim: inline-start inline-end block-start } \
         .i { width: 4; height: 1; margin: 1 }",
        &["", "", ""],
    );
    // Line 1: [0 1] — 4 + 1 + 1 + 4 = 10 ≤ 12 (the first item's start
    // and the last one's end margins trimmed); line 2: [2] alone.
    assert_eq!(xs(&r), [0, 6, 0]);
    // The first line's block-start margins are trimmed; the second's
    // top margin stays (the first line is 1 + 1 tall with its bottom
    // margins).
    assert_eq!(ys(&r), [0, 0, 3]);
    // Stretched items show each line's cross size: the first line's is
    // 0 + 1 + 1, the second's 1 + 1 + 1.
    let r = items(
        ".f { flex-wrap: wrap; width: 12; margin-trim: inline-start inline-end block-start } \
         .i { width: 4; margin: 1 }",
        &["", "", ""],
    );
    assert_eq!(ys(&r), [0, 0, 3]);
    assert_eq!(r.iter().map(|r| r.3).collect::<Vec<_>>(), [1, 1, 1]);
    // Under `wrap-reverse` the first line is at the bottom, so it is its
    // block-end margins that adjoin the container's edge (§5.2).
    let r = items(
        ".f { flex-wrap: wrap-reverse; width: 12; margin-trim: block-end } \
         .i { width: 4; height: 1; margin: 1 }",
        &["", "", ""],
    );
    assert_eq!(ys(&r), [4, 4, 1]);
}

/// CSSOM View §4: a multi-line container's lines overflow its cross
/// axis and scroll; under `wrap-reverse` the scrolling area origin is
/// the cross-start edge — the bottom of a row — so the overflow past
/// the top is reached with a negative `scrollTop`.
#[test]
fn wrapped_lines_scroll() {
    for (wrap, first_y, max_scroll) in [("wrap", 0, (0, 1)), ("wrap-reverse", 1, (-1, 0))] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "div", "f");
        let ids: Vec<_> = (0..5).map(|_| el(&mut dom, f, "div", "i")).collect();
        lay_out(
            &mut dom,
            &format!(
                ".f {{ display: flex; flex-wrap: {wrap}; width: 10; height: 2; overflow-y: auto }} \
                 .i {{ width: 4; height: 1; flex-shrink: 0 }}"
            ),
            30,
            20,
        );
        assert_eq!(rect(&dom, ids[0]).y, first_y, "{wrap}");
        // The last line, past the far end, is reached by scrolling.
        dom.node_mut(f).set_scroll_top(-50).unwrap();
        assert_eq!(dom.node(f).scroll_top(), Some(max_scroll.0), "{wrap}");
        dom.node_mut(f).set_scroll_top(50).unwrap();
        assert_eq!(dom.node(f).scroll_top(), Some(max_scroll.1), "{wrap}");
        let to_end = if wrap == "wrap" { 1 } else { -1 };
        dom.node_mut(f).set_scroll_top(to_end).unwrap();
        dom.layout_dom(rdom_tui::render::Rect::new(0, 0, 30, 20));
        assert_eq!(
            rect(&dom, ids[4]).y,
            if wrap == "wrap" { 1 } else { 0 },
            "{wrap}"
        );
    }
}
