//! C6G-FLEX-BASIS-ZERO — `flex: <n>` takes a basis of `0%`, as every
//! engine does (CSS Flexbox §7.2 says `0`): a percentage basis against
//! an indefinite container main size behaves as `content` (§9.2 step 3,
//! §7.3.3), so an `auto`-height column of `flex: 1` items sizes them by
//! their content.

use super::{el, lay_out, size};
use rdom_tui::{NodeId, TuiDom};

fn rows_of(dom: &mut TuiDom, parent: NodeId, n: usize) -> NodeId {
    let item = el(dom, parent, "div", "i");
    for _ in 0..n {
        let r = el(dom, item, "div", "");
        let t = dom.create_text_node("x");
        dom.append_child(r, t).unwrap();
    }
    item
}

/// CSS Flexbox §7.3.3: a percentage `flex-basis` "is resolved against
/// the flex item's containing block (i.e. its flex container); and if
/// that containing block's size is indefinite, the used value for
/// `flex-basis` is `content`" — an `auto`-height column's items, one
/// row and three rows of content, are 1 and 3 rows, not an equal split.
#[test]
fn flex_n_items_of_an_auto_height_column_size_by_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let a = rows_of(&mut dom, c, 1);
    let b = rows_of(&mut dom, c, 3);
    lay_out(
        &mut dom,
        ".c { display: flex; flex-direction: column } .i { flex: 1; min-height: 0 }",
        10,
        8,
    );
    assert_eq!((size(&dom, a).1, size(&dom, b).1), (1, 3));
    assert_eq!(size(&dom, c).1, 4);
}

/// In a definite container main size `0%` is 0: a row's `flex: 1` items
/// share it equally, whatever their content.
#[test]
fn flex_n_items_of_a_definite_row_share_it_equally() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let a = el(&mut dom, c, "div", "i");
    let t = dom.create_text_node("abcdef");
    dom.append_child(a, t).unwrap();
    let b = el(&mut dom, c, "div", "i");
    lay_out(
        &mut dom,
        ".c { display: flex; width: 10 } .i { flex: 1; min-width: 0 }",
        10,
        2,
    );
    assert_eq!((size(&dom, a).0, size(&dom, b).0), (5, 5));
}
