//! C6-DIRECTION-REVERSE — `flex-direction: row-reverse | column-reverse`
//! (CSS Flexbox §5.1): main-start and main-end swap, with `direction`
//! (CSS Writing Modes 4 §2.1) deciding which physical edge a row's
//! main-start is.

use super::{el, lay_out, rect};
use rdom_tui::{LayoutExt, TuiAccessors, TuiAccessorsMut, TuiDom};

/// Three 2-cell items in a 10 × 6 flex container under `css`; their x
/// and y.
fn items(css: &str) -> Vec<(i32, i32)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|c| el(&mut dom, f, "div", &format!("i {c}")))
        .collect();
    lay_out(
        &mut dom,
        &format!(".f {{ display: flex; width: 10; height: 6 }} .i {{ width: 2; height: 1 }} {css}"),
        12,
        8,
    );
    ids.iter()
        .map(|&n| (rect(&dom, n).x, rect(&dom, n).y))
        .collect()
}

/// CSS Flexbox §5.1: `row-reverse` places the first item at main-end of
/// the row — its main-start is the right edge under `ltr`, the left one
/// under `rtl` (the two mirrors cancel).
#[test]
fn row_reverse_starts_at_the_inline_end() {
    let xs = |css: &str| items(css).into_iter().map(|(x, _)| x).collect::<Vec<_>>();
    assert_eq!(xs(".f { flex-direction: row-reverse }"), [8, 6, 4]);
    assert_eq!(
        xs(".f { flex-direction: row-reverse; direction: rtl }"),
        [0, 2, 4]
    );
    assert_eq!(xs(".f { flex-direction: row; direction: rtl }"), [8, 6, 4]);
}

/// CSS Flexbox §5.1: `column-reverse` stacks from the bottom edge.
#[test]
fn column_reverse_starts_at_the_bottom() {
    let ys = items(".f { flex-direction: column-reverse }")
        .into_iter()
        .map(|(_, y)| y)
        .collect::<Vec<_>>();
    assert_eq!(ys, [5, 4, 3]);
}

/// CSS Flexbox §5.1 / §8.1: an item's main-start margin is on the
/// main-start side — the right margin in `row-reverse`, the bottom
/// margin in `column-reverse`.
#[test]
fn main_start_margins_follow_the_reversal() {
    let first = |css| items(css)[0];
    assert_eq!(
        first(".f { flex-direction: row-reverse } .a { margin-right: 3; margin-left: 1 }").0,
        10 - 3 - 2
    );
    assert_eq!(
        first(".f { flex-direction: column-reverse } .a { margin-bottom: 2; margin-top: 1 }").1,
        6 - 2 - 1
    );
}

/// CSS Box 4 §3.2: `margin-trim: inline-end` trims the margin adjoining
/// the inline-end edge — in `row-reverse` (`ltr`) the first item's
/// main-start (right) margin.
#[test]
fn margin_trim_follows_the_reversal() {
    let first = items(
        ".f { flex-direction: row-reverse; margin-trim: inline-end } \
         .i { margin-left: 1; margin-right: 1 }",
    )[0];
    assert_eq!(first.0, 10 - 2);

    // CSS Sizing 3 §5.2: the container's max-content width drops the
    // trimmed margin — the first item's right one, not the last's.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    el(&mut dom, f, "div", "i a");
    el(&mut dom, f, "div", "i");
    lay_out(
        &mut dom,
        ".f { display: flex; flex-direction: row-reverse; margin-trim: inline-end; \
              width: max-content } \
         .i { width: 2; height: 1; margin-left: 1; margin-right: 1 } .a { margin-right: 3 }",
        20,
        3,
    );
    assert_eq!(rect(&dom, f).width, (1 + 2) + (1 + 2 + 1));
}

/// CSSOM View §4 with CSS Flexbox §5.1: a reversed container's
/// scrolling area origin is its main-start edge, so it starts scrolled
/// to that end and its overflow (past main-end) is reached with a
/// negative offset — `scrollLeft` in `row-reverse`, `scrollTop` in
/// `column-reverse`, as in current browsers.
#[test]
fn reversed_containers_scroll_from_main_start() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let r = el(&mut dom, root, "div", "r");
    let ra = el(&mut dom, r, "div", "w");
    let rb = el(&mut dom, r, "div", "w");
    let c = el(&mut dom, root, "div", "c");
    let ca = el(&mut dom, c, "div", "h");
    let cb = el(&mut dom, c, "div", "h");
    lay_out(
        &mut dom,
        ".r { display: flex; flex-direction: row-reverse; overflow-x: auto; width: 4; height: 2 } \
         .w { width: 3; height: 1; flex-shrink: 0 } \
         .c { display: flex; flex-direction: column-reverse; overflow-y: auto; width: 4; height: 2 } \
         .h { height: 2; flex-shrink: 0 }",
        10,
        8,
    );
    assert_eq!((rect(&dom, ra).x, rect(&dom, rb).x), (1, -2));
    let c_top = rect(&dom, c).y;
    assert_eq!((rect(&dom, ca).y, rect(&dom, cb).y), (c_top, c_top - 2));
    assert_eq!(dom.node(r).scroll_left(), Some(0));
    assert_eq!(dom.node(c).scroll_top(), Some(0));

    dom.node_mut(r).set_scroll_left(-2).unwrap();
    dom.node_mut(c).set_scroll_top(-2).unwrap();
    dom.layout_dom(rdom_tui::render::Rect::new(0, 0, 10, 8));
    assert_eq!(dom.node(r).scroll_left(), Some(-2));
    assert_eq!(dom.node(c).scroll_top(), Some(-2));
    assert_eq!(rect(&dom, rb).x, 0);
    assert_eq!(rect(&dom, cb).y, c_top);

    dom.node_mut(c).set_scroll_top(-50).unwrap();
    assert_eq!(dom.node(c).scroll_top(), Some(-2), "clamped at the far end");
    dom.node_mut(c).set_scroll_top(3).unwrap();
    assert_eq!(dom.node(c).scroll_top(), Some(0), "clamped at the origin");
}
