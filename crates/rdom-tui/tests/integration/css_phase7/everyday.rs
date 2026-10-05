//! C7G-DOCS-TESTS — the everyday grids, end to end: the `1fr` column
//! that keeps a long word's width where `minmax(0, 1fr)` does not,
//! `place-items: center`, and the `auto 1fr auto` page under an `auto`
//! and a `min-height` container height.

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// Two items `.a` (text `a_text`) and `.b` (`x`) in a 10-wide grid laid
/// out with `css`; returns each one's border box `(x, width)`.
fn two_columns(css: &str, a_text: &str) -> [(i32, u16); 2] {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let mut item = |class: &str, text: &str| -> NodeId {
        let id = el(&mut dom, g, "div", class);
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
        id
    };
    let (a, b) = (item("a", a_text), item("b", "x"));
    lay_out(&mut dom, css, 10, 4);
    [a, b].map(|id| {
        let r = rect(&dom, id);
        (r.x, r.width)
    })
}

/// CSS Grid 2 §7.2.4 and §6.6: `1fr` is `minmax(auto, 1fr)`, and an
/// `auto` minimum is the item's automatic minimum size — its min-content
/// width, the unbreakable word — so the `fr` columns cannot shrink the
/// word's column below 10 and the second is left its content (§11.7:
/// there is no free space to distribute). `minmax(0, 1fr)` drops the
/// floor and the two columns share the 10 cells; so does a scroll
/// container, whose automatic minimum is 0 (§6.6).
#[test]
fn a_long_word_holds_its_fr_column_open_unless_the_minimum_is_zero() {
    let word = "abcdefghij";
    assert_eq!(
        two_columns(".g { display: grid; grid-template-columns: 1fr 1fr }", word),
        [(0, 10), (10, 1)]
    );
    assert_eq!(
        two_columns(
            ".g { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) }",
            word
        ),
        [(0, 5), (5, 5)]
    );
    assert_eq!(
        two_columns(
            ".g { display: grid; grid-template-columns: 1fr 1fr } .a { overflow: auto }",
            word
        ),
        [(0, 5), (5, 5)]
    );
}

/// CSS Box Alignment 3 §6.4 with CSS Grid 2 §10.3–§10.4: `place-items:
/// center` sets `align-items` and `justify-items`, so an `auto`-sized
/// item takes its content size, `fit-content`, and is centered in its
/// area on both axes: `ab` in a 10 × 5 cell at (4, 2).
#[test]
fn place_items_center_centers_each_item_in_its_area() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let i = el(&mut dom, g, "div", "");
    let t = dom.create_text_node("ab");
    dom.append_child(i, t).unwrap();
    lay_out(
        &mut dom,
        ".g { display: grid; grid-template: 5 / 10; place-items: center }",
        20,
        8,
    );
    let r = rect(&dom, i);
    assert_eq!((r.x, r.y, r.width, r.height), (4, 2, 2, 1));
}

/// The page layout, rows `auto 1fr auto` with `gap: 1` (CSS Grid 2
/// §7.2, §11.7, Box Alignment 3 §8), each row one line of text. Under
/// an `auto` height the grid's height is indefinite, so the `1fr` row
/// is sized like `max-content` (§11.7.1: the flex fraction is the
/// largest of the items' max-content contributions over their flex
/// factor) — 1 — and the page is 5 rows. Under `min-height: 9` the grid
/// is 9 tall and the `fr` row takes the space the other rows and the
/// gaps leave (§11.7: "if … the grid container has a definite min-height
/// …, the leftover space"), 5, so the footer is on the last row.
#[test]
fn the_auto_1fr_auto_page_with_gaps_fills_a_min_height() {
    let at = |extra: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let g = el(&mut dom, root, "div", "g");
        let ids: Vec<NodeId> = ["head", "main", "foot"]
            .iter()
            .map(|text| {
                let id = el(&mut dom, g, "div", "");
                let t = dom.create_text_node(text);
                dom.append_child(id, t).unwrap();
                id
            })
            .collect();
        lay_out(
            &mut dom,
            &format!(".g {{ display: grid; grid-template-rows: auto 1fr auto; gap: 1; {extra} }}"),
            20,
            12,
        );
        let page = rect(&dom, g).height;
        let rows: Vec<(i32, u16)> = ids
            .iter()
            .map(|&id| {
                let r = rect(&dom, id);
                (r.y, r.height)
            })
            .collect();
        (page, rows)
    };
    assert_eq!(at(""), (5, vec![(0, 1), (2, 1), (4, 1)]));
    assert_eq!(at("min-height: 9"), (9, vec![(0, 1), (2, 5), (8, 1)]));
}
