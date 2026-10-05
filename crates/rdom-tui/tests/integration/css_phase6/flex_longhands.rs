//! C6-FLEX-LONGHANDS — `flex-grow`, `flex-basis` and the `flex`
//! shorthand's basis in layout: the flex base size (CSS Flexbox §9.2
//! step 3) and the resolution of flexible lengths (§9.7).

use super::{el, lay_out, rect};
use rdom_tui::TuiDom;

/// Items `.i0`, `.i1`, … (one per entry of `texts`, each holding its
/// text) in a `row` flex container `.f`, laid out under `css`; their
/// widths and x positions.
fn row(css: &str, texts: &[&str]) -> Vec<(u16, i32)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let id = el(&mut dom, f, "div", &format!("i i{i}"));
            if !t.is_empty() {
                let n = dom.create_text_node(t);
                dom.append_child(id, n).unwrap();
            }
            id
        })
        .collect();
    lay_out(
        &mut dom,
        &format!(".f {{ display: flex; flex-direction: row; width: 12; height: 1 }} {css}"),
        20,
        4,
    );
    ids.iter()
        .map(|&n| (rect(&dom, n).width, rect(&dom, n).x))
        .collect()
}

fn widths(css: &str, texts: &[&str]) -> Vec<u16> {
    row(css, texts).into_iter().map(|(w, _)| w).collect()
}

/// §7.3.1 / §9.7: the positive free space is shared by `flex-grow`.
#[test]
fn flex_grow_shares_the_free_space() {
    assert_eq!(
        widths(
            ".i { width: 2 } .i0 { flex-grow: 1 } .i1 { flex-grow: 2 }",
            &["", "", ""]
        ),
        [4, 6, 2]
    );
}

/// §7.3.3 / §9.2 step 3: a definite `flex-basis` is the flex base size —
/// over `width` — and a percentage resolves against the container's
/// inner main size.
#[test]
fn a_definite_flex_basis_is_the_base_size() {
    assert_eq!(
        widths(
            ".i { width: 2; flex-shrink: 0 } .i0 { flex-basis: 4 } .i1 { flex-basis: 50% } \
             .i2 { flex-basis: calc(25% - 2) }",
            &["", "", ""]
        ),
        [4, 6, 1]
    );
}

/// §9.2 step 3: `flex-basis: content` (and `auto` with an `auto` main
/// size) sizes from the max-content size; `auto` with a definite `width`
/// uses the width.
#[test]
fn content_and_auto_bases() {
    assert_eq!(
        widths(
            ".i { flex-shrink: 0 } .i0 { width: 2 } .i1 { width: 2; flex-basis: content }",
            &["abcdef", "abcdef", "abc"]
        ),
        [2, 6, 3]
    );
}

/// §7.2 / §9.7: `flex: 1 1 auto` grows from the content size, `flex: 1`
/// (basis 0) from nothing.
#[test]
fn the_shorthand_basis_is_laid_out() {
    assert_eq!(
        widths(".i { flex: 1 1 auto }", &["aaaa", "bb"]),
        [4 + 3, 2 + 3]
    );
    assert_eq!(widths(".i { flex: 1 }", &["aaaa", "bb"]), [6, 6]);
}

/// CSS UI 3 §3.1 (`box-sizing`) with Flexbox §7.3.3: `flex-basis`
/// measures the box `box-sizing` names, as `width` does.
#[test]
fn flex_basis_follows_box_sizing() {
    assert_eq!(
        widths(
            ".i { flex-basis: 4; padding: 0 1; flex-shrink: 0 } .i1 { box-sizing: border-box }",
            &["", ""]
        ),
        [6, 4]
    );
}

/// CSS Sizing 3 §3.1 with Flexbox §7.3.3: the intrinsic keywords as
/// `flex-basis`.
#[test]
fn intrinsic_keyword_bases() {
    assert_eq!(
        widths(
            ".i { flex-shrink: 0 } .i0 { flex-basis: min-content } \
             .i1 { flex-basis: max-content }",
            &["aaa bbbb", "aaa bb"]
        ),
        [4, 6]
    );
}

/// §4.5 with §9.7: a `flex: 1` item's automatic minimum size is its
/// min-content size — it does not grow to less than that; its sibling
/// takes the rest.
#[test]
fn a_zero_basis_item_keeps_its_automatic_minimum() {
    assert_eq!(widths(".i { flex: 1 }", &["aaaaaaaa", "b"]), [8, 4]);
}

/// §9.7 step 4.c: shrinking is shared by the scaled shrink factor —
/// `flex-shrink` × flex base size.
#[test]
fn shrinking_is_weighted_by_the_base_size() {
    assert_eq!(
        widths(
            ".i { min-width: 0 } .i0 { flex-basis: 8 } .i1 { flex-basis: 16 }",
            &["", ""]
        ),
        [8 - 4, 16 - 8]
    );
}

/// §9.7 then §8.1: the free space goes to growing items first; the
/// `auto` margins take only what is left.
#[test]
fn growing_items_take_the_free_space_before_auto_margins() {
    let items = row(
        ".i { width: 2 } .i0 { flex-grow: 1 } .i1 { margin-left: auto }",
        &["", ""],
    );
    assert_eq!(items, [(10, 0), (2, 10)]);
}
