//! C6G-COLLAPSE — `visibility: collapse` on a flex item against CSS
//! Flexbox §4.4 and §9.4 step 10: layout runs with the item uncollapsed,
//! notes the cross size of the line it is in as its strut size, and runs
//! again with the item a zero-main-size strut that the rest of the
//! algorithm ignores — gaps and `justify-content` included — but for
//! keeping its line at least its strut size.

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

fn item(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let id = el(dom, parent, "span", class);
    let t = dom.create_text_node(text);
    dom.append_child(id, t).unwrap();
    id
}

/// §9.4 step 10: the strut size is the cross size of the item's line as
/// laid out uncollapsed — `a b c` at its hypothetical main size, 5
/// wide, is one row, not the three it wraps to at the strut's zero
/// main size.
#[test]
fn the_strut_is_the_uncollapsed_lines_cross_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    item(&mut dom, f, "", "x");
    let c = item(&mut dom, f, "c", "a b c");
    lay_out(
        &mut dom,
        ".f { display: flex } .c { visibility: collapse }",
        10,
        4,
    );
    assert_eq!(rect(&dom, f).height, 1);
    assert_eq!((rect(&dom, c).width, rect(&dom, c).height), (0, 1));
}

/// §9.4 step 10: after the strut sizes are noted, collapsed items are
/// ignored "as if they were `display: none`" — no gap is placed beside
/// one (CSS Box Alignment 3 §8.1 spaces the items of the line).
#[test]
fn no_gap_is_placed_beside_a_collapsed_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = item(&mut dom, f, "", "a");
    item(&mut dom, f, "c", "c");
    let b = item(&mut dom, f, "", "b");
    lay_out(
        &mut dom,
        ".f { display: flex; gap: 2 } .c { visibility: collapse }",
        10,
        2,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, b).x), (0, 3));
}

/// §9.4 step 10 with §8.2: `justify-content` distributes the free space
/// among the items it does not ignore — `space-between` puts the second
/// of two visible items at the end, whatever strut follows it.
#[test]
fn justify_content_ignores_a_collapsed_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = item(&mut dom, f, "", "a");
    let b = item(&mut dom, f, "", "b");
    item(&mut dom, f, "c", "c");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 10; justify-content: space-between } \
         .c { visibility: collapse }",
        10,
        2,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, b).x), (0, 9));
}
