//! C7-GRID-CORE — the grid container (CSS Grid 2 §5) and its items
//! (§6): `display: grid` / `inline-grid`, the box tree of a grid
//! container, its auto-placed items, its size, and its scrolling.

use super::{el, lay_out, paint, rect, rows, size};
use rdom_tui::render::{InlineFlow, inline_flow_for_text};
use rdom_tui::{Display, HitTestExt, TuiAccessors, TuiDom, TuiNodeExt};

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, data: &str) -> rdom_tui::NodeId {
    let t = dom.create_text_node(data);
    dom.append_child(parent, t).unwrap();
    t
}

/// An element `tag.class` holding `data`.
fn holding(
    dom: &mut TuiDom,
    parent: rdom_tui::NodeId,
    class: &str,
    data: &str,
) -> rdom_tui::NodeId {
    let e = el(dom, parent, "span", class);
    text(dom, e, data);
    e
}

/// CSS Grid 2 §5.1 / §7.2: `display: grid` makes a grid container whose
/// columns are `grid-template-columns`; §8.5 auto-places the items in
/// order, filling each row before starting the next, and the rows past
/// the explicit grid are implicit `auto` rows (§7.5 / §7.6).
#[test]
fn a_grid_container_places_its_items_in_its_columns() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = holding(&mut dom, g, "", "A");
    let b = holding(&mut dom, g, "", "B");
    let c = holding(&mut dom, g, "", "C");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 3 5 }",
        10,
        3,
    );
    assert_eq!(
        rows(&buf, 10, 3),
        ["A  B      ", "C         ", "          "]
    );
    for (id, want) in [(a, (0, 0, 3, 1)), (b, (3, 0, 5, 1)), (c, (0, 1, 3, 1))] {
        let r = rect(&dom, id);
        assert_eq!((r.x, r.y, r.width, r.height), want);
    }
    assert_eq!(size(&dom, g), (10, 2), "as tall as its two rows");
}

/// CSS Display 3 §2.7, CSS Grid 2 §6.1: a grid item's `display` is
/// blockified — an inline `span` item computes to `block`.
#[test]
fn grid_items_are_blockified() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let s = holding(&mut dom, g, "", "x");
    lay_out(&mut dom, ".g { display: inline-grid }", 10, 2);
    assert_eq!(dom.node(s).computed().unwrap().display, Display::Block);
}

/// CSS Grid 2 §6.1: "each contiguous sequence of child text runs is
/// wrapped in an anonymous block container grid item"; a run of only
/// white space is not rendered, so it is no item.
#[test]
fn text_runs_are_anonymous_grid_items() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    text(&mut dom, g, "ab");
    let s = holding(&mut dom, g, "", "c");
    text(&mut dom, g, "   ");
    let t = holding(&mut dom, g, "", "d");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 3 3 3 }",
        9,
        1,
    );
    assert_eq!(rows(&buf, 9, 1), ["ab c  d  "]);
    assert_eq!((rect(&dom, s).x, rect(&dom, t).x), (3, 6));
}

/// CSS Grid 2 §6.1 with CSS Pseudo-Elements 4 §4: a grid container's
/// `::before` / `::after` are child boxes, so grid items; a text-only
/// grid container's text is its one anonymous item, not inline content.
#[test]
fn pseudo_elements_are_grid_items() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    text(&mut dom, g, "x");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 2 2 2 } \
         .g::before { content: '[' } .g::after { content: ']' }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["[ x ] "]);
}

/// CSS Grid 2 §8.5 (via CSS Display 3 §3, `order`): auto-placement
/// takes the items in order-modified document order.
#[test]
fn order_reorders_grid_items() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = holding(&mut dom, g, "", "a");
    let b = holding(&mut dom, g, "first", "b");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 2 2 } .first { order: -1 }",
        4,
        1,
    );
    assert_eq!(rows(&buf, 4, 1), ["b a "]);
    assert_eq!((rect(&dom, b).x, rect(&dom, a).x), (0, 2));
}

/// CSS Grid 2 §5.1: `inline-grid` is an inline-level grid container —
/// an atomic inline in its line (CSS Display 3 §2.4) — and its width is
/// its max-content size, the sum of its tracks sized under a
/// max-content constraint (§5.2): `auto auto` over `x` and `yz` is 3.
#[test]
fn inline_grid_is_an_atom_its_tracks_wide() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text(&mut dom, p, "ab");
    let g = el(&mut dom, p, "span", "g");
    let x = holding(&mut dom, g, "", "x");
    let yz = holding(&mut dom, g, "", "yz");
    text(&mut dom, p, "cd");
    let buf = paint(
        &mut dom,
        ".g { display: inline-grid; grid-template-columns: auto auto }",
        10,
        1,
    );
    assert_eq!(rows(&buf, 10, 1), ["abxyzcd   "]);
    let r = rect(&dom, g);
    assert_eq!((r.x, r.width, r.height), (2, 3, 1));
    assert_eq!((rect(&dom, x).x, rect(&dom, yz).x), (2, 3));
}

/// CSS Grid 2 §5.2: a grid container's min-content / max-content size
/// is the sum of its tracks sized under that constraint. Under a
/// max-content constraint an `auto` column takes its item's max-content
/// size (§11.6, infinite free space) and a `2fr` column the flex
/// fraction its item needs (§11.7, indefinite free space): `ab cd` and
/// `e f` give 5 + 3; under a min-content constraint the tracks keep
/// their min-content base sizes and the flex fraction is zero: 2 + 1.
#[test]
fn a_grid_container_measures_its_tracks() {
    for (width, want) in [("max-content", 8), ("min-content", 3)] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let g = el(&mut dom, root, "div", "g");
        let a = holding(&mut dom, g, "", "ab cd");
        let b = holding(&mut dom, g, "", "e f");
        lay_out(
            &mut dom,
            &format!(".g {{ display: grid; width: {width}; grid-template-columns: auto 2fr }}"),
            20,
            4,
        );
        assert_eq!(size(&dom, g).0, want, "{width}");
        if width == "max-content" {
            // Laid out at 8 cells: the `auto` column's base 2 grows to its
            // limit 5 (§11.6), the `2fr` column takes the 3 left (§11.7).
            assert_eq!((rect(&dom, a).width, rect(&dom, b).x), (5, 5));
        }
    }
}

/// CSS Grid 2 §5.2 / §11.1: a block-level grid container's auto height
/// is the sum of its rows and row gaps, so the block after it starts
/// below them.
#[test]
fn a_grid_container_is_as_tall_as_its_rows() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "");
    let g = el(&mut dom, wrap, "div", "g");
    for t in ["a", "b", "c"] {
        holding(&mut dom, g, "", t);
    }
    let after = holding(&mut dom, wrap, "after", "z");
    lay_out(
        &mut dom,
        ".g { display: grid; row-gap: 1 } .after { display: block }",
        10,
        8,
    );
    assert_eq!(size(&dom, g).1, 5, "three rows, two gaps");
    assert_eq!(rect(&dom, after).y, 5);
}

/// CSS Grid 2 §5.3 / CSS Overflow 3 §2: a grid container is a scroll
/// container like any other; its scrollable overflow is its grid.
#[test]
fn a_grid_container_scrolls_its_grid() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    for t in ["a", "b", "c", "d"] {
        holding(&mut dom, g, "", t);
    }
    lay_out(
        &mut dom,
        ".g { display: grid; height: 2; overflow-y: scroll; grid-template-rows: 1 1 1 1 }",
        10,
        4,
    );
    assert_eq!(dom.node(g).scroll_height(), Some(4));
    assert_eq!(dom.node(g).scroll_range().unwrap().y(), 0..=2);
}

/// CSS Grid 2 §7.1 with CSS Writing Modes 4 §2.1: the column axis is
/// the inline axis, so under `direction: rtl` column 1 is the right one.
#[test]
fn rtl_grid_columns_run_right_to_left() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = holding(&mut dom, g, "", "a");
    let b = holding(&mut dom, g, "", "b");
    lay_out(
        &mut dom,
        ".g { display: grid; direction: rtl; grid-template-columns: 2 3 }",
        10,
        1,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, b).x), (8, 5));
    assert_eq!((rect(&dom, a).width, rect(&dom, b).width), (2, 3));
}

/// CSS Sizing 3 §3.1 with CSS Grid 2 §5.2: `width: fit-content` clamps
/// the available space between the grid's min-content size (2 + 2) and
/// its max-content size (5 + 5) — 7 here — and the grid lays out in it:
/// the `auto` columns' bases 2 and 2 share the 3 free cells, the odd one
/// to the first (§11.6, DIVERGENCES §1).
#[test]
fn a_fit_content_grid_clamps_its_width_between_its_content_sizes() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = holding(&mut dom, g, "", "ab cd");
    let b = holding(&mut dom, g, "", "ef gh");
    lay_out(
        &mut dom,
        ".g { display: grid; width: fit-content; grid-template-columns: auto auto }",
        7,
        4,
    );
    assert_eq!(size(&dom, g).0, 7);
    assert_eq!(
        (rect(&dom, a).width, rect(&dom, b).x, rect(&dom, b).width),
        (4, 4, 3)
    );
}

/// CSS Grid 2 §5.2 / CSS Flexbox §9.2: a grid container that is a flex
/// item contributes its max-content size, the sum of its fixed columns.
#[test]
fn a_grid_flex_item_is_its_tracks_wide() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let g = el(&mut dom, f, "div", "g");
    holding(&mut dom, g, "", "a");
    let after = holding(&mut dom, f, "", "z");
    lay_out(
        &mut dom,
        ".f { display: flex } .g { display: grid; grid-template-columns: 3 4 }",
        20,
        2,
    );
    assert_eq!(size(&dom, g).0, 7);
    assert_eq!(rect(&dom, after).x, 7);
}

/// Hit-testing and the caret reach a grid's anonymous items (CSS Grid 2
/// §6.1): a point on one resolves into its text node, and each text's
/// inline flow is its item's box — indexed in the container's item
/// sequence (its `::before`, its children, its `::after`).
#[test]
fn the_caret_resolves_into_each_anonymous_grid_item() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ab = text(&mut dom, g, "ab");
    holding(&mut dom, g, "", "x");
    let cd = text(&mut dom, g, "cd");
    lay_out(
        &mut dom,
        ".g { display: grid; grid-template-columns: 3 3 3 3 } .g::before { content: '' }",
        12,
        1,
    );
    // `::before` (empty) | `ab` | `x` | `cd`.
    let at = dom.position_at(4, 0).expect("a position on `ab`");
    assert_eq!((at.node, at.offset), (ab, 1));
    let at = dom.position_at(10, 0).expect("a position on `cd`");
    assert_eq!((at.node, at.offset), (cd, 1));
    assert_eq!(
        inline_flow_for_text(&dom, cd),
        Some(InlineFlow::Anonymous {
            container: g,
            index: 2
        })
    );
}

/// CSS Grid 2 §6.2: a grid item's containing block is its grid area,
/// whose size the track sizing algorithm makes definite (§11.1), so a
/// percentage height inside an `auto`-height item resolves: half of a
/// 4-row area is 2.
#[test]
fn percentages_inside_a_grid_item_resolve_against_its_area() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let item = el(&mut dom, g, "div", "");
    let half = el(&mut dom, item, "div", "half");
    lay_out(
        &mut dom,
        ".g { display: grid; grid-template-rows: 4 } .half { height: 50% }",
        10,
        6,
    );
    assert_eq!(size(&dom, item).1, 4);
    assert_eq!(size(&dom, half).1, 2);
}

/// CSS Grid 2 §6.1: "A grid item establishes an independent formatting
/// context for its contents" — its first child's top margin stays inside
/// it, where block layout would let it collapse through.
#[test]
fn a_grid_item_contains_its_childrens_margins() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let item = el(&mut dom, g, "div", "");
    let p = holding(&mut dom, item, "p", "x");
    lay_out(
        &mut dom,
        ".g { display: grid } .p { display: block; margin-top: 2 }",
        10,
        6,
    );
    assert_eq!((rect(&dom, item).y, rect(&dom, p).y), (0, 2));
    assert_eq!(size(&dom, item).1, 3);
}

/// CSS Box 4 §3 (`margin-trim`) on a grid container: the margins of the
/// items adjoining a trimmed edge of the grid — in its first row for
/// `block-start`, its first column for `inline-start` — are zero, in
/// sizing the tracks and in placing the items.
#[test]
fn margin_trim_drops_the_margins_at_the_grid_edges() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let ids: Vec<_> = ["a", "b", "c", "d"]
        .iter()
        .map(|t| holding(&mut dom, g, "i", t))
        .collect();
    lay_out(
        &mut dom,
        ".g { display: grid; margin-trim: block-start inline-start; \
         grid-template-columns: 3 3 } .i { margin: 1 }",
        10,
        8,
    );
    let at: Vec<_> = ids
        .iter()
        .map(|&id| {
            let r = rect(&dom, id);
            (r.x, r.y, r.width)
        })
        .collect();
    // Row 1 is its items' text and bottom margins (2); row 2 adds the top
    // margins (3).
    assert_eq!(at, [(0, 0, 2), (4, 0, 1), (0, 3, 2), (4, 3, 1)]);
    assert_eq!(size(&dom, g).1, 5);
}

/// CSS Grid 2 §6.3 / §6.5: grid items paint in order-modified document
/// order — of two overlapping items the one later in that order paints
/// on top and is hit first — whatever the tree order.
#[test]
fn grid_items_paint_and_hit_in_order_modified_document_order() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = holding(&mut dom, g, "a", "aa");
    let b = holding(&mut dom, g, "", "bb");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 2 2 } .a { order: 1; margin-left: -1 }",
        4,
        1,
    );
    // `b` is placed first, `a` second and pulled one cell over `b`.
    assert_eq!(rows(&buf, 4, 1), ["baa "]);
    assert_eq!(dom.hit_test(1, 0), Some(a));
    assert_eq!(dom.hit_test(0, 0), Some(b));
}

/// CSS Grid 2 §6.5: a grid item paints "exactly the same as inline
/// blocks" — atomically, as if it created a stacking context (CSS 2.1
/// Appendix E): its `box-shadow` spilling over the item before it lies
/// over that item's text, the later item painting whole after it.
#[test]
fn a_grid_item_paints_atomically() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    holding(&mut dom, g, "", "aaaa");
    holding(&mut dom, g, "b", "b");
    let buf = paint(
        &mut dom,
        ".g { display: grid; grid-template-columns: 4 2 } .b { box-shadow: -2 0 0 0 red }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["aa  b "]);
    let red = rdom_tui::Color::Rgb(255, 0, 0);
    assert_eq!(buf.cell(2, 0).unwrap().bg, red);
    assert_ne!(buf.cell(1, 0).unwrap().bg, red);
}

/// C7G-INLINE-ATOM-MAX — CSS Sizing 3 §5.1: the max-content inline size
/// of inline content is its widest line laid out with no wrapping, and an
/// atomic inline (CSS Display 3 §2.4: `inline-grid`, `inline-flex`,
/// `inline-block`) is a box in that line, its own max-content width
/// wide (CSS 2.1 §10.3.9), not the text inside it. `x ` beside an atom
/// 10 wide is 12: a `width: max-content` paragraph holds it on one line,
/// whether its line is an anonymous block box's or its own.
#[test]
fn an_atoms_box_counts_in_its_lines_max_content() {
    for atom in [
        "display: inline-grid; grid-template-columns: 5 5",
        "display: inline-flex; width: 10",
        "display: inline-block; width: 10",
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "p", "p");
        text(&mut dom, p, "x ");
        let a = holding(&mut dom, p, "a", "a");
        lay_out(
            &mut dom,
            &format!(".p {{ width: max-content }} .a {{ {atom} }}"),
            30,
            4,
        );
        assert_eq!(size(&dom, p), (12, 1), "{atom}");
        assert_eq!(rect(&dom, a).x, 2, "{atom}");
    }
    // Beside an inline box the paragraph is an inline formatting context
    // of its own (not an anonymous block box's): `x y ` and the atom, 14.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "p");
    text(&mut dom, p, "x ");
    holding(&mut dom, p, "", "y");
    text(&mut dom, p, " ");
    holding(&mut dom, p, "a", "a");
    lay_out(
        &mut dom,
        ".p { width: max-content } .a { display: inline-grid; grid-template-columns: 5 5 }",
        30,
        4,
    );
    assert_eq!(size(&dom, p), (14, 1), "in an inline formatting context");
}
