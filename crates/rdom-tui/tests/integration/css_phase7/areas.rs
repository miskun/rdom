//! C7-GRID-AREAS — named grid areas (CSS Grid 2 §7.3): the explicit
//! grid's size from `grid-template-areas` (§7.1), the lines each named
//! area names implicitly (§7.3.2), and items placed by area name through
//! `grid-area` (§8.3, §8.4).

use super::{el, lay_out, rect};
use rdom_tui::{NodeId, TuiDom};

/// A grid `.g` holding one `div.<class>` per entry (text `x`), laid out
/// with `css` at 20 × 10. Returns each item's border box `(x, y, width,
/// height)`.
fn place(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids: Vec<NodeId> = classes
        .iter()
        .map(|c| {
            let s = el(&mut dom, g, "div", c);
            let t = dom.create_text_node("x");
            dom.append_child(s, t).unwrap();
            s
        })
        .collect();
    lay_out(&mut dom, css, 20, 10);
    ids.iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

const LAYOUT: &str = ".g { display: grid; grid-template-columns: 3 5 4; \
     grid-template-rows: 1 2 1; \
     grid-template-areas: \"head head head\" \"nav main .\" \"foot foot foot\" } \
     .h { grid-area: head } .n { grid-area: nav } .m { grid-area: main } \
     .f { grid-area: foot }";

/// §7.3 with §8.3: `grid-area: <name>` sets all four lines to the name,
/// and a lone `<custom-ident>` matches the line `<name>-start` /
/// `<name>-end` the named area gives its edges (§7.3.2), so each item
/// fills its area.
#[test]
fn items_are_placed_by_area_name() {
    let at = place(LAYOUT, &["f", "m", "n", "h"]);
    assert_eq!(
        at,
        [(0, 3, 12, 1), (3, 1, 5, 2), (0, 1, 3, 2), (0, 0, 12, 1)]
    );
}

/// §7.3.2: each named area gives its row-start and column-start lines
/// the name `<area>-start` and its end lines `<area>-end`; they place
/// items as any other line name does, in a line, alone (`<ident>` then
/// matching the first line of that name, there being no
/// `<ident>-start`) or as a span's target.
#[test]
fn named_areas_name_their_lines() {
    let at = place(
        &format!(
            "{LAYOUT} .a {{ grid-column: nav-end / head-end; grid-row: main-start }} \
             .b {{ grid-row: foot-start; grid-column: 1 / span main-end }}"
        ),
        &["a", "b"],
    );
    // `nav-end` is line 2 and `head-end` line 4: columns 2–3, 9 cells;
    // `main-start` alone is the row line 2, spanning one row.
    // `b` spans from line 1 to the next line named `main-end`, line 3.
    assert_eq!(at, [(3, 1, 9, 2), (0, 3, 8, 1)]);
}

/// §7.1: "the size of the explicit grid is determined by the larger of
/// the number of rows/columns defined by `grid-template-areas` and the
/// number of rows/columns sized by `grid-template-rows` /
/// `grid-template-columns`. Any rows/columns defined by
/// `grid-template-areas` but not sized by `grid-template-rows` /
/// `grid-template-columns` take their size from the `grid-auto-rows` /
/// `grid-auto-columns` properties." Auto-placement fills the explicit
/// grid's three columns before starting a new row, and a negative line
/// counts from the explicit grid's end, which the areas set.
#[test]
fn the_areas_size_the_explicit_grid() {
    let at = place(
        ".g { display: grid; grid-template-columns: 2; grid-auto-columns: 3; \
         grid-template-areas: \"a b c\" } .last { grid-column: -2 }",
        &["", "", "", "last"],
    );
    assert_eq!(at, [(0, 0, 2, 1), (2, 0, 3, 1), (5, 0, 3, 1), (5, 1, 3, 1)]);
}

/// §9.1 with §7.3: an absolutely positioned child of a grid container is
/// placed in the grid area its `grid-area` names.
#[test]
fn an_absolutely_positioned_box_fills_its_named_area() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let abs = el(&mut dom, g, "div", "abs");
    lay_out(
        &mut dom,
        &format!(
            "{LAYOUT} .g {{ position: relative }} \
             .abs {{ position: absolute; inset: 0; grid-area: main }}"
        ),
        20,
        6,
    );
    let r = rect(&dom, abs);
    assert_eq!((r.x, r.y, r.width, r.height), (3, 1, 5, 2));
}
