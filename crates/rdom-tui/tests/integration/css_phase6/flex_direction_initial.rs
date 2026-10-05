//! C6-FLEX-DIRECTION-INITIAL — `flex-direction`'s initial value is `row`
//! (CSS Flexbox §5.1), and a block container's block axis comes from its
//! formatting context, not from `flex-direction` (CSS Display 3 §2: the
//! property applies to flex containers only).

use super::{el, lay_out, rect, size};
use rdom_tui::TuiDom;

/// §5.1: `flex-direction: row` is the initial value — a flex container
/// that does not set it lays its items out along the inline axis.
#[test]
fn a_flex_container_without_flex_direction_is_a_row() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = (0..3).map(|_| el(&mut dom, f, "div", "i")).collect();
    lay_out(
        &mut dom,
        ".f { display: flex; width: 20; height: 4 } .i { width: 2 }",
        24,
        6,
    );
    let at: Vec<_> = ids
        .iter()
        .map(|&n| (rect(&dom, n).x, rect(&dom, n).y))
        .collect();
    assert_eq!(at, [(0, 0), (2, 0), (4, 0)]);
    // §9.4 step 11: the single line's items stretch to its cross size.
    assert_eq!(size(&dom, ids[0]), (2, 4));
}

/// `flex-direction` applies to flex containers only (§5.1 "Applies to:
/// flex containers"): a block container stacks its block children along
/// its block axis whatever the property says, and measures them that way
/// — its max-content width is its widest child's (CSS Sizing 3 §5.1), not
/// their sum.
#[test]
fn flex_direction_does_not_turn_a_block_container_into_a_row() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let b = el(&mut dom, f, "div", "b");
    let kids: Vec<_> = (0..2).map(|_| el(&mut dom, b, "div", "k")).collect();
    lay_out(
        &mut dom,
        ".f { display: flex; flex-direction: row; width: 20; height: 4 } \
         .b { flex-direction: row } .k { width: 3; height: 1 }",
        24,
        6,
    );
    // The block, a flex item with an `auto` width, is as wide as its
    // widest child (§9.2 step 3: its max-content size).
    assert_eq!(size(&dom, b).0, 3);
    let at: Vec<_> = kids
        .iter()
        .map(|&n| (rect(&dom, n).x, rect(&dom, n).y))
        .collect();
    assert_eq!(at, [(0, 0), (0, 1)]);
}

/// The document root's children stack top to bottom (rdom's viewport
/// column, a browser's `<body>` of blocks) whatever `flex-direction`'s
/// initial value is.
#[test]
fn the_document_roots_children_stack() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", "a");
    let b = el(&mut dom, root, "div", "a");
    lay_out(&mut dom, ".a { height: 2 }", 10, 6);
    assert_eq!((rect(&dom, a).y, rect(&dom, b).y), (0, 2));
    assert_eq!(size(&dom, a).0, 10);
}
