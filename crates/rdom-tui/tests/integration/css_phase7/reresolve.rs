//! C7-GRID-RERESOLVE — CSS Grid 2 §11.1 steps 3–4: an item whose
//! min-content contribution to the columns changes once the rows are
//! sized has the columns sized again, and the rows after them, once
//! each. In a terminal an item's width depends on its height only
//! through a preferred aspect ratio (CSS Sizing 4 §5.1: a definite block
//! size transfers through it), so these grids hold one.

use super::{el, lay_out, rect};
use rdom_tui::TuiDom;

/// `.g` holding an empty `div.r` and a `div.t` with `text`, laid out with
/// `css` at 20 × 12. Returns the rects of `.g`, `.r` and `.t` as `(x, y,
/// width, height)`.
fn grid(css: &str, text: &str) -> [(i32, i32, u16, u16); 3] {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let r = el(&mut dom, g, "div", "r");
    let t = el(&mut dom, g, "div", "t");
    let txt = dom.create_text_node(text);
    dom.append_child(t, txt).unwrap();
    lay_out(&mut dom, css, 20, 12);
    [g, r, t].map(|id| {
        let b = rect(&dom, id);
        (b.x, b.y, b.width, b.height)
    })
}

/// CSS Sizing 4 §5.1 with Grid §11.5: an item with a preferred aspect
/// ratio and a definite height contributes that height through the ratio
/// to its column — `height: 4; aspect-ratio: 2` is 8 wide, and so is its
/// `auto` column.
#[test]
fn a_ratio_item_sizes_its_column_from_a_definite_height() {
    let [_, r, t] = grid(
        ".g { display: grid; grid-template-columns: auto 1fr } \
         .r { height: 4; aspect-ratio: 2 }",
        "x",
    );
    assert_eq!((r.2, r.3), (8, 4));
    assert_eq!(t.0, 8);
}

/// §11.1 step 3: "If the min-content contribution of any grid item has
/// changed based on the row sizes and alignment calculated in step 2,
/// re-resolve the sizes of the grid columns with the new min-content and
/// max-content contributions (once only)." `height: 100%` has no basis
/// while the columns are first sized, and is 3 once the 3-row track is:
/// the ratio makes the item 6 wide, and its column follows. A stretched
/// height (`align-self: stretch`) is definite the same way.
#[test]
fn the_columns_are_sized_again_after_the_rows() {
    let at = |r: &str| {
        grid(
            &format!(
                ".g {{ display: grid; grid-template-columns: auto 1fr; \
                 grid-template-rows: 3 }} .r {{ aspect-ratio: 2; {r} }}"
            ),
            "x",
        )
    };
    let [_, r, t] = at("height: 100%");
    assert_eq!((r.2, r.3, t.0), (6, 3, 6));
    let [_, r, t] = at("align-self: stretch");
    assert_eq!((r.2, r.3, t.0), (6, 3, 6));
}

/// §11.1 step 4: "if the min-content contribution of any grid item has
/// changed based on the column sizes … calculated in step 3, re-resolve
/// the sizes of the grid rows … (once only)". Step 2 sizes the `auto`
/// row to `.t`'s two lines at 20 wide; step 3 makes `.r` (`height:
/// 100%`, ratio 5) 10 wide, which leaves `.t` 10 columns, where it wraps
/// to three lines — step 4 sizes the row again, to 3, and the grid with
/// it.
#[test]
fn the_rows_are_sized_again_after_the_columns() {
    let [g, _, t] = grid(
        ".g { display: grid; grid-template-columns: auto 1fr } \
         .r { height: 100%; aspect-ratio: 5 }",
        "aaaa bbbb cccc dddd eeee",
    );
    assert_eq!((t.0, t.2, t.3), (10, 10, 3));
    assert_eq!(g.3, 3);
}
