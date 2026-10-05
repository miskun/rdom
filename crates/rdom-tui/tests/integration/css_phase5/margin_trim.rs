//! C5-MARGIN-TRIM — `margin-trim` (CSS Box 4 §3) on block and flex
//! containers.

use super::{el, lay_out, rect, size};
use rdom_tui::{NodeId, TuiDom};

/// `div.c` (child of a block `div.wrap`) holding `div.a` and `div.b`,
/// laid out under `css` in 30 × 12.
fn container(css: &str) -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let c = el(&mut dom, wrap, "div", "c");
    let a = el(&mut dom, c, "div", "a");
    let b = el(&mut dom, c, "div", "b");
    lay_out(&mut dom, css, 30, 12);
    (dom, c, a, b)
}

// ── block containers ───────────────────────────────────────────────

/// CSS Box 4 §3: `block-start` truncates to zero the block-start margin
/// of the block-level child adjoining the container's block-start
/// content edge; `block-end` the block-end margin adjoining its end — so
/// an `auto` height no longer holds it.
#[test]
fn block_container_trims_its_edge_children_margins() {
    let base = ".c { border: solid } .a, .b { height: 1; margin: 2 0 }";
    let (dom, c, a, b) = container(base);
    assert_eq!(
        (rect(&dom, a).y, rect(&dom, b).y, size(&dom, c).1),
        (3, 6, 10)
    );
    let (dom, c, a, b) = container(&format!("{base} .c {{ margin-trim: block-start }}"));
    assert_eq!(
        (rect(&dom, a).y, rect(&dom, b).y, size(&dom, c).1),
        (1, 4, 8)
    );
    let (dom, c, a, b) = container(&format!("{base} .c {{ margin-trim: block }}"));
    assert_eq!(
        (rect(&dom, a).y, rect(&dom, b).y, size(&dom, c).1),
        (1, 4, 6)
    );
}

/// CSS Box 4 §3: a trimmed margin is gone, so it does not collapse
/// through the container either (CSS 2.1 §8.3.1 would carry the first
/// child's margin out through a container with no border or padding).
#[test]
fn a_trimmed_margin_does_not_collapse_through() {
    let base = ".a { height: 1; margin-top: 2 } .b { height: 1 }";
    let (dom, c, a, _) = container(base);
    assert_eq!((rect(&dom, c).y, rect(&dom, a).y), (2, 2));
    let (dom, c, a, _) = container(&format!("{base} .c {{ margin-trim: block-start }}"));
    assert_eq!((rect(&dom, c).y, rect(&dom, a).y), (0, 0));
}

/// CSS Box 4 §3: on a block container only the block-axis values trim;
/// `inline` leaves the children's inline margins alone (every child
/// adjoins both inline edges, and a block container does not trim
/// them — see DIVERGENCES).
#[test]
fn block_container_ignores_inline_trim() {
    let (dom, _, a, _) = container(".c { margin-trim: inline } .a { height: 1; margin-left: 2 }");
    assert_eq!(rect(&dom, a).x, 2);
}

// ── flex containers ────────────────────────────────────────────────

/// CSS Box 4 §3.2: in a flex container the main-start margin of the
/// first item and the main-end margin of the last are trimmed by the
/// container's main-axis start / end values, and every item's cross
/// margins by its cross-axis values (one flex line). For a `row` the
/// main axis is the inline axis.
#[test]
fn flex_row_trims_inline_ends_and_block_sides() {
    let base = ".c { display: flex; flex-direction: row; height: 4 } \
                .a, .b { width: 2; margin: 1; flex-shrink: 0 }";
    let (dom, _, a, b) = container(base);
    assert_eq!(
        (
            rect(&dom, a).x,
            rect(&dom, b).x,
            rect(&dom, a).y,
            size(&dom, a).1
        ),
        (1, 5, 1, 2)
    );
    let (dom, _, a, b) = container(&format!("{base} .c {{ margin-trim: inline block }}"));
    assert_eq!(
        (
            rect(&dom, a).x,
            rect(&dom, b).x,
            rect(&dom, a).y,
            size(&dom, a).1
        ),
        (0, 4, 0, 4)
    );
    let (dom, _, a, b) = container(&format!(
        "{base} .c {{ margin-trim: inline-end block-end }}"
    ));
    assert_eq!(
        (
            rect(&dom, a).x,
            rect(&dom, b).x,
            rect(&dom, a).y,
            size(&dom, a).1
        ),
        (1, 5, 1, 3)
    );
}

/// CSS Box 4 §3.2: for a `column` the main axis is the block axis —
/// `block-start` trims the first item's top margin, `inline` every
/// item's left and right.
#[test]
fn flex_column_trims_block_ends_and_inline_sides() {
    let base = ".c { display: flex; flex-direction: column; width: 10 } \
                .a, .b { height: 1; margin: 1 }";
    let (dom, _, a, b) = container(&format!(
        "{base} .c {{ margin-trim: block-start inline-start inline-end }}"
    ));
    assert_eq!((rect(&dom, a).y, rect(&dom, b).y), (0, 3));
    assert_eq!((rect(&dom, a).x, size(&dom, a).0), (0, 10));
}

/// A shrink-to-fit container's size leaves out the trimmed margins
/// (CSS Box 4 §3: they are zero for layout, intrinsic sizes included).
#[test]
fn intrinsic_size_leaves_out_trimmed_margins() {
    let (dom, c, _, _) = container(
        ".c { position: absolute; top: 0; left: 0; display: flex; flex-direction: row; \
              margin-trim: inline } \
         .a, .b { width: 2; height: 1; margin: 0 3 }",
    );
    assert_eq!(size(&dom, c).0, 2 + 3 + 3 + 2);
}
