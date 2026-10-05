//! C6-ALIGN-CONTENT — `align-content` in flex layout (CSS Flexbox §8.4,
//! §9.4 step 15; CSS Box Alignment 3 §5.1 / §5.4): a multi-line
//! container's free cross space placed before its lines or spread
//! between them; no effect on a single-line container.

use super::{el, lay_out, rect, size};
use rdom_tui::TuiDom;

/// `n` 4 × 1 items in a wrapping flex container `.f` (10 wide; a line
/// holds two) under `css`; their y positions.
fn ys_of(n: usize, css: &str) -> Vec<i32> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..n).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        &format!(
            ".f {{ display: flex; flex-wrap: wrap; width: 10; height: 10 }} \
             .i {{ width: 4; height: 1 }} {css}"
        ),
        20,
        14,
    );
    ids.iter().map(|&i| rect(&dom, i).y).collect()
}

fn ys(css: &str) -> Vec<i32> {
    ys_of(3, css)
}

/// §8.4: the lines pack toward cross-start / cross-end / the center (the
/// leading space rounded down); `start` / `end` follow the writing mode
/// (Box Alignment §4.2). `normal` behaves as `stretch` (§5.3): the lines
/// share the free space (C6-WRAP).
#[test]
fn positions_pack_the_lines() {
    for v in ["flex-start", "start"] {
        assert_eq!(
            ys(&format!(".f {{ align-content: {v} }}")),
            [0, 0, 1],
            "{v}"
        );
    }
    for v in ["flex-end", "end"] {
        assert_eq!(
            ys(&format!(".f {{ align-content: {v} }}")),
            [8, 8, 9],
            "{v}"
        );
    }
    assert_eq!(ys(".f { align-content: center }"), [4, 4, 5]);
    for v in ["normal", "stretch"] {
        assert_eq!(
            ys(&format!(".f {{ align-content: {v} }}")),
            [0, 0, 5],
            "{v}"
        );
    }
}

/// §8.4 / Box Alignment §4.3: `space-between` / `space-around` /
/// `space-evenly` spread the free space between the lines — whole cells,
/// each line at its rolling position rounded up.
#[test]
fn distributions_space_the_lines() {
    assert_eq!(ys(".f { align-content: space-between }"), [0, 0, 9]);
    assert_eq!(ys(".f { align-content: space-around }"), [2, 2, 7]);
    assert_eq!(ys(".f { align-content: space-evenly }"), [3, 3, 7]);
}

/// §8.4: negative free space — `center` overflows both ways (rounded
/// down), `safe` aligns as `start`, `space-between` falls back to `safe
/// flex-start` and `space-around` / `space-evenly` to `safe center`.
#[test]
fn overflowing_lines() {
    let short = ".f { height: 1; width: 4 }";
    assert_eq!(
        ys(&format!("{short} .f {{ align-content: center }}")),
        [-1, 0, 1]
    );
    assert_eq!(
        ys(&format!("{short} .f {{ align-content: safe center }}")),
        [0, 1, 2]
    );
    for v in ["space-between", "space-around", "space-evenly"] {
        assert_eq!(
            ys(&format!("{short} .f {{ align-content: {v} }}")),
            [0, 1, 2],
            "{v}"
        );
    }
}

/// §5.2 with Box Alignment §4.2: under `wrap-reverse` `flex-start` is
/// the bottom edge, `start` stays the top.
#[test]
fn wrap_reverse_flips_flex_start() {
    let w = ".f { flex-wrap: wrap-reverse }";
    assert_eq!(
        ys(&format!("{w} .f {{ align-content: flex-start }}")),
        [9, 9, 8]
    );
    assert_eq!(ys(&format!("{w} .f {{ align-content: start }}")), [1, 1, 0]);
}

/// §8.4 "has no effect on a single-line flex container" — as every
/// current browser does (the CSSWG kept it for web compatibility,
/// csswg-drafts#3052): a `nowrap` line is the container's size. A
/// `wrap` container with one line is multi-line, and aligns it.
#[test]
fn single_line_containers_ignore_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "f a");
    let ai = el(&mut dom, a, "div", "i");
    let b = el(&mut dom, root, "div", "f b");
    let bi = el(&mut dom, b, "div", "i");
    for n in [ai, bi] {
        let t = dom.create_text_node("x");
        dom.append_child(n, t).unwrap();
    }
    lay_out(
        &mut dom,
        ".f { display: flex; width: 10; height: 5; align-content: center } \
         .b { flex-wrap: wrap }",
        20,
        14,
    );
    assert_eq!((rect(&dom, ai).y, size(&dom, ai).1), (0, 5));
    assert_eq!(
        (rect(&dom, bi).y - rect(&dom, b).y, size(&dom, bi).1),
        (2, 1)
    );
}

/// A column's lines are columns: `align-content` places them along the
/// inline axis.
#[test]
fn a_columns_lines_spread_horizontally() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..3).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        ".f { display: flex; flex-flow: column wrap; width: 10; height: 2; \
         align-content: center } .i { width: 2; height: 2 }",
        20,
        14,
    );
    let xs: Vec<_> = ids.iter().map(|&i| rect(&dom, i).x).collect();
    assert_eq!(xs, [2, 4, 6]);
}
