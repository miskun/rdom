//! C6G-FLEX-SPEC — the flex algorithm's corner cases against CSS
//! Flexbox §4.5 (the automatic minimum size), §9.2 (the flex base size)
//! and §9.7 (resolving the flexible lengths).

use super::{el, lay_out, size};
use rdom_tui::{NodeId, TuiDom};

fn item(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let id = el(dom, parent, "div", class);
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    id
}

/// CSS Flexbox §4.5: "In all cases, the size is clamped by the maximum
/// main size if it's definite" — a `max-width: 3` item whose content is
/// 8 wide shrinks to 3, not to its content.
#[test]
fn the_automatic_minimum_is_clamped_by_the_max_main_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = item(&mut dom, f, "i", "abcdefgh");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 2 } .i { max-width: 3 }",
        10,
        2,
    );
    assert_eq!(size(&dom, i).0, 3);
}

/// CSS Flexbox §9.7 step 1: the free space is decided by the sum of the
/// outer *hypothetical* main sizes — the flex base size clamped by the
/// min and max main sizes, the §4.5 automatic minimum included. A
/// `flex: 1 1 0` item holding `abcdefgh` is 8 hypothetically, so the
/// line (8 + 4 in 10) shrinks: the `min-width: 0` item gives up 2.
#[test]
fn the_hypothetical_main_size_includes_the_automatic_minimum() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = item(&mut dom, f, "a", "abcdefgh");
    let b = item(&mut dom, f, "b", "");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 10 } .a { flex: 1 1 0 } .b { width: 4; min-width: 0 }",
        10,
        2,
    );
    assert_eq!((size(&dom, a).0, size(&dom, b).0), (8, 2));
}

/// CSS Flexbox §9.7 step 4.c: "multiply its flex shrink factor by its
/// inner flex base size" — the content box, not the border box. Over 12
/// cells, a 10-wide item and a 2-wide one with 2 cells of padding a side
/// (border box 6) take the 4 cells of overflow 10 : 2 — 3 and 1.
#[test]
fn the_scaled_shrink_factor_uses_the_inner_flex_base_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = item(&mut dom, f, "a", "");
    let b = item(&mut dom, f, "b", "");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 12 } \
         .a { width: 10; min-width: 0 } \
         .b { box-sizing: content-box; width: 2; padding: 0 2; min-width: 0 }",
        20,
        2,
    );
    assert_eq!((size(&dom, a).0, size(&dom, b).0), (7, 5));
}

/// CSS Flexbox §9.7: a line with no room — its inner main size 0 —
/// still shrinks its items (the free space is negative), down to their
/// min main sizes.
#[test]
fn a_line_with_no_room_still_shrinks_its_items() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = item(&mut dom, f, "i", "");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 0 } .i { width: 5; min-width: 0 }",
        10,
        2,
    );
    assert_eq!(size(&dom, i).0, 0);
}

/// CSS Display 3 §2.7: a flex item is blockified — an `inline-block`
/// item is block-level — so `align-items: normal` stretches it (CSS
/// Flexbox §9.4 step 11).
#[test]
fn an_inline_block_item_is_stretched() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = item(&mut dom, f, "i", "x");
    lay_out(
        &mut dom,
        ".f { display: flex; height: 3 } .i { display: inline-block }",
        10,
        3,
    );
    assert_eq!(size(&dom, i).1, 3);
}

/// CSS Flexbox §9.2 step 3.B: an item with a preferred aspect ratio, a
/// used flex basis of `content` (`auto` with an `auto` width) and a
/// definite cross size takes its flex base size from that cross size
/// through the ratio — `aspect-ratio: 2` at `height: 3` is 6 wide.
#[test]
fn the_flex_base_size_comes_from_the_aspect_ratio_and_a_definite_cross_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = item(&mut dom, f, "i", "");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 10; align-items: flex-start } \
         .i { aspect-ratio: 2; height: 3 }",
        10,
        4,
    );
    assert_eq!(size(&dom, i), (6, 3));
}
