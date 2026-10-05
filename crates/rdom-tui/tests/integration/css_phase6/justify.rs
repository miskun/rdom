//! C6-JUSTIFY — `justify-content` in flex layout (CSS Box Alignment 3
//! §5.2 / §6, CSS Flexbox §8.2): the line's leftover free space, after
//! the flexible lengths and `auto` margins, placed by position or
//! distributed between the items, in whole cells.

use super::{el, lay_out, rect};
use rdom_tui::TuiDom;

/// `n` 2-cell items in a flex container `.f` (`width` cells wide) under
/// `css`; their x positions.
fn xs_of(n: usize, width: u16, css: &str) -> Vec<i32> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..n).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        &format!(
            ".f {{ display: flex; width: {width}; height: 6 }} \
             .i {{ width: 2; height: 1; flex-shrink: 0 }} {css}"
        ),
        30,
        10,
    );
    ids.iter().map(|&i| rect(&dom, i).x).collect()
}

fn xs(css: &str) -> Vec<i32> {
    xs_of(3, 10, css)
}

/// Flexbox §8.2: `flex-start` packs toward main-start; `normal` behaves
/// as `stretch`, which behaves as `flex-start` in flex (Box Alignment
/// §5.3 / §6.1); `flex-end` packs toward main-end; `center` splits the
/// free space — whole cells, the leading space rounded down.
#[test]
fn positional_values_pack_the_line() {
    for v in ["normal", "flex-start", "stretch", "start", "left"] {
        assert_eq!(
            xs(&format!(".f {{ justify-content: {v} }}")),
            [0, 2, 4],
            "{v}"
        );
    }
    for v in ["flex-end", "end", "right"] {
        assert_eq!(
            xs(&format!(".f {{ justify-content: {v} }}")),
            [4, 6, 8],
            "{v}"
        );
    }
    assert_eq!(xs(".f { justify-content: center }"), [2, 4, 6]);
    assert_eq!(xs_of(3, 11, ".f { justify-content: center }"), [2, 4, 6]);
}

/// Box Alignment §4.3: `space-between` puts the free space between the
/// items, `space-around` half-size spaces at the ends, `space-evenly`
/// equal spaces everywhere. Whole cells: each space is the step between
/// rolling positions rounded up, so a remainder cell goes to each of the
/// first spaces.
#[test]
fn distribution_values_space_the_items() {
    assert_eq!(xs(".f { justify-content: space-between }"), [0, 4, 8]);
    assert_eq!(
        xs_of(3, 11, ".f { justify-content: space-between }"),
        [0, 5, 9]
    );
    assert_eq!(xs(".f { justify-content: space-around }"), [1, 4, 8]);
    assert_eq!(xs(".f { justify-content: space-evenly }"), [1, 4, 7]);
    // The gaps come first (Box Alignment §8.1): 10 − 6 − 2 = 2 free.
    assert_eq!(
        xs(".f { justify-content: space-between; column-gap: 1 }"),
        [0, 4, 8]
    );
}

/// Flexbox §8.2: with a single item, or negative free space,
/// `space-between` falls back to `safe flex-start` and `space-around` /
/// `space-evenly` (Box Alignment §4.3) to `safe center`.
#[test]
fn distribution_falls_back() {
    assert_eq!(xs_of(1, 10, ".f { justify-content: space-between }"), [0]);
    assert_eq!(xs_of(1, 10, ".f { justify-content: space-around }"), [4]);
    assert_eq!(xs_of(1, 10, ".f { justify-content: space-evenly }"), [4]);
    for v in ["space-between", "space-around", "space-evenly"] {
        assert_eq!(
            xs_of(3, 4, &format!(".f {{ justify-content: {v} }}")),
            [0, 2, 4],
            "{v}"
        );
    }
}

/// Box Alignment §4.4: an overflowing line is aligned as asked — `center`
/// overflows both ends (the leading space rounded down), `flex-end` the
/// start — unless `safe`, which aligns it as `start`.
#[test]
fn overflow_positions() {
    assert_eq!(xs_of(3, 4, ".f { justify-content: center }"), [-1, 1, 3]);
    assert_eq!(xs_of(3, 3, ".f { justify-content: center }"), [-2, 0, 2]);
    assert_eq!(
        xs_of(3, 4, ".f { justify-content: unsafe center }"),
        [-1, 1, 3]
    );
    assert_eq!(
        xs_of(3, 4, ".f { justify-content: safe center }"),
        [0, 2, 4]
    );
    assert_eq!(xs_of(3, 4, ".f { justify-content: flex-end }"), [-2, 0, 2]);
    assert_eq!(
        xs_of(3, 4, ".f { justify-content: safe flex-end }"),
        [0, 2, 4]
    );
}

/// `flex-start` / `flex-end` follow the main axis (CSS Flexbox §5.1);
/// `start` / `end` the inline axis of the writing mode, `left` / `right`
/// the physical edges (Box Alignment §4.2). The items keep their main-axis
/// order.
#[test]
fn start_end_left_right_follow_their_axes() {
    let rtl = ".f { direction: rtl }";
    assert_eq!(
        xs(&format!("{rtl} .f {{ justify-content: flex-start }}")),
        [8, 6, 4]
    );
    assert_eq!(
        xs(&format!("{rtl} .f {{ justify-content: start }}")),
        [8, 6, 4]
    );
    assert_eq!(
        xs(&format!("{rtl} .f {{ justify-content: left }}")),
        [4, 2, 0]
    );
    assert_eq!(
        xs(&format!("{rtl} .f {{ justify-content: right }}")),
        [8, 6, 4]
    );
    let rev = ".f { flex-direction: row-reverse }";
    assert_eq!(
        xs(&format!("{rev} .f {{ justify-content: flex-start }}")),
        [8, 6, 4]
    );
    assert_eq!(
        xs(&format!("{rev} .f {{ justify-content: start }}")),
        [4, 2, 0]
    );
    assert_eq!(
        xs(&format!("{rev} .f {{ justify-content: flex-end }}")),
        [4, 2, 0]
    );
    assert_eq!(
        xs(&format!("{rev} .f {{ justify-content: end }}")),
        [8, 6, 4]
    );
    assert_eq!(
        xs(&format!("{rev} .f {{ justify-content: left }}")),
        [4, 2, 0]
    );
}

/// In a column the main axis is the block axis: `left` / `right` are
/// not on it and behave as `start` (Box Alignment §4.2); `end` packs to
/// the bottom.
#[test]
fn a_column_justifies_vertically() {
    let ys = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "div", "f");
        let ids: Vec<_> = (0..2).map(|_| el(&mut dom, f, "div", "i")).collect();
        lay_out(
            &mut dom,
            &format!(
                ".f {{ display: flex; flex-direction: column; width: 4; height: 6 }} \
                 .i {{ height: 1 }} {css}"
            ),
            10,
            10,
        );
        ids.iter().map(|&i| rect(&dom, i).y).collect::<Vec<_>>()
    };
    assert_eq!(ys(".f { justify-content: right }"), [0, 1]);
    assert_eq!(ys(".f { justify-content: end }"), [4, 5]);
    assert_eq!(ys(".f { justify-content: space-between }"), [0, 5]);
}

/// Flexbox §8.1: `auto` margins take the free space first, so
/// `justify-content` has none left to place.
#[test]
fn auto_margins_take_the_free_space_first() {
    assert_eq!(
        xs(".f { justify-content: center } .i:first-child { margin-left: auto }"),
        [4, 6, 8]
    );
}

/// §8.2 aligns each line on its own: a two-line container centers both.
#[test]
fn each_line_is_justified() {
    assert_eq!(
        xs_of(4, 7, ".f { flex-wrap: wrap; justify-content: center }"),
        [0, 2, 4, 2]
    );
}
