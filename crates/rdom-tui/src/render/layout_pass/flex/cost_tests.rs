//! C6G-FLEX-COST: what one flex layout measures and allocates — the
//! §4.5 automatic minimum resolved once per item and only where it can
//! bind, each item's cross size measured once, and the §9.7 freeze loop
//! allocating nothing per iteration.

use std::cell::Cell;

use rdom_core::NodeId;

use crate::layout::Direction;
use crate::render::Rect;
use crate::render::layout_pass::intrinsic::memo_tests::COLUMN_WALKS;
use crate::{CascadeExt, LayoutExt, TuiDom};

thread_local! {
    /// §4.5 automatic minimums resolved (`distribute::resolve_auto_min`).
    pub(super) static AUTO_MINS: Cell<usize> = const { Cell::new(0) };
}

/// A flex container `.f` holding one `div` per class, each with the
/// text `abcdefgh`, cascaded from `css`.
fn container(css: &str, classes: &[&str]) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = dom.create_element("div");
    dom.set_attribute(f, "class", "f").unwrap();
    dom.append_child(root, f).unwrap();
    let items = classes
        .iter()
        .map(|c| {
            let id = dom.create_element("div");
            dom.set_attribute(id, "class", c).unwrap();
            dom.append_child(f, id).unwrap();
            let t = dom.create_text_node("abcdefgh");
            dom.append_child(id, t).unwrap();
            id
        })
        .collect();
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    dom.cascade(&sheet);
    (dom, f, items)
}

/// CSS Flexbox §4.5 / §9.3 / §9.7: in a multi-line row, a `flex: 1 1 0`
/// item's automatic minimum (its content, 8) sets its hypothetical size
/// for line breaking and floors its flexed size — one resolution serves
/// both, in each of the two runs of the flex algorithm (the container's
/// height measured by its parent, then its layout; the content walk
/// behind it is memoized for the pass); a `width: 6` item's cannot
/// exceed its base (the specified size suggestion is that width), so it
/// is never resolved. A non-stretched item's cross size is measured
/// once, at its used width.
#[test]
fn a_multi_line_row_measures_each_item_once() {
    let (mut dom, _, items) = container(
        ".f { display: flex; flex-wrap: wrap; width: 10; align-items: flex-start } \
         .g { flex: 1 1 0 } .w { flex: 0 1 auto; width: 6 }",
        &["g", "g", "w", "w"],
    );
    AUTO_MINS.with(|c| c.set(0));
    COLUMN_WALKS.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 20, 10));
    let widths: Vec<u16> = items
        .iter()
        .map(|&i| {
            crate::node::TuiNodeExt::layout_rect(&dom.node(i))
                .unwrap()
                .width
        })
        .collect();
    assert_eq!(widths, [10, 10, 6, 6]);
    assert_eq!(AUTO_MINS.with(Cell::get), 2 * 2, "automatic minimums");
    // One per item, plus the container's own height (the viewport
    // column measures it).
    assert!(
        COLUMN_WALKS.with(Cell::get) <= items.len() + 1,
        "{} Column measurements",
        COLUMN_WALKS.with(Cell::get)
    );
}

/// The §9.7 freeze loop's allocations do not grow with its iterations:
/// three items with no clamp freeze in one; with `max-width: 2` and
/// `max-width: 12` they take three (the first clamps, then the second).
#[test]
fn the_freeze_loop_allocates_nothing_per_iteration() {
    use super::distribute::{MainAxisBudget, resolve_flexible_lengths};
    use super::main_axis::{MainBudgets, collect_main_axis_items};
    let allocations = |css: &str, expect: [u16; 3]| {
        let (dom, f, items) = container(css, &["a", "b", "c"]);
        let budgets = MainBudgets { main: 30, cross: 5 };
        let trim = crate::render::layout_pass::margin_trim::FlexTrim::of(
            &crate::style::ComputedStyle::initial(),
            Direction::Row,
        );
        let infos = collect_main_axis_items(&dom, &items, Direction::Row, budgets, trim, false);
        let _ = f;
        let mut sizes = Vec::new();
        let n = crate::test_alloc::allocations_in(|| {
            sizes = resolve_flexible_lengths(
                &dom,
                &infos,
                Direction::Row,
                MainAxisBudget {
                    main: 30,
                    cross: 5,
                    net: 30,
                },
            );
        });
        assert_eq!(sizes, expect, "{css}");
        n
    };
    let one = allocations(
        ".f { display: flex } .a, .b, .c { flex: 1 1 0; min-width: 0 }",
        [10, 10, 10],
    );
    let three = allocations(
        ".f { display: flex } .a, .b, .c { flex: 1 1 0; min-width: 0 } \
         .a { max-width: 2 } .b { max-width: 12 }",
        [2, 12, 16],
    );
    assert_eq!(
        three, one,
        "allocations: {one} in one iteration, {three} in three"
    );
    // The targets and the frozen flags.
    assert_eq!(one, 2);
}
