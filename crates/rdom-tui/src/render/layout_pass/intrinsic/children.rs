//! The content size of a box with in-flow element children (CSS Sizing
//! 3 §5.1 / §5.2, CSS Flexbox §9.9): their outer contributions summed
//! where they flow along the queried axis (plus gaps), the largest
//! where they stack across it, a multi-line flex container's lines on
//! its cross axis, and direct text runs and line-box pseudo-elements
//! beside them — before the box's own padding, border and inline
//! pseudo-elements, which `measure_content` adds.

use rdom_core::{Dom, NodeId, NodeType};

use crate::render::layout_pass::flex::item::FlexItem;

use super::inline::{border_main_cost, own_line_pseudo_rows};
use super::{IntrinsicMode, Measure, intrinsic_size_inner, intrinsic_text, wrap};
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::style::ComputedStyle;

/// `id`'s in-flow `children` — its element children, or a flex
/// container's flex items, anonymous ones included (CSS Flexbox §4) —
/// measured along `direction`, with a block container's direct text
/// runs; `cross_budget` is the extent `id` is measured
/// against on the other axis, `containing_block_width` the basis of its
/// own padding percentages.
#[allow(clippy::too_many_arguments)]
pub(super) fn children_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    children: &[FlexItem],
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    measure: Measure,
) -> u16 {
    let cb_w_for_pad = containing_block_width;
    // Cross budget to forward to children. They'll be laid out
    // inside our content area; for IFC measurement at the child
    // level this is what determines wrap.
    // Padding percentages resolve against the containing block's width
    // on both axes (CSS Box 3 §4.2), not against the budget.
    let child_cross_budget = match direction {
        Direction::Row => cross_budget.saturating_sub(
            computed
                .padding
                .vertical(cb_w_for_pad)
                .saturating_add(border_main_cost(computed, Direction::Column)),
        ),
        Direction::Column => cross_budget.saturating_sub(
            computed
                .padding
                .horizontal(cb_w_for_pad)
                .saturating_add(border_main_cost(computed, Direction::Row)),
        ),
    };

    // The children's containing block is this element's content box.
    // Measuring along the Row axis, that width is the very thing being
    // computed — a cyclic percentage, which CSS Sizing 3 §5.2.1 treats
    // as zero for the intrinsic contribution. Along the Column axis it
    // is the content width the children will wrap to.
    let child_cb_width = match direction {
        Direction::Row => 0,
        Direction::Column => child_cross_budget,
    };

    // Flexbox §9.9 / §4.5: an item's contribution is its outer size —
    // margins on the queried axis included.
    // CSS Box 4 §3 `margin-trim`: a trimmed edge drops the adjoining
    // margins — of the first / last child where the children flow
    // along the queried axis, of every child where they stack across it.
    // Under `rtl` the first child's inline-start margin is its right one
    // (CSS Writing Modes 4 §2.1).
    let along = crate::render::layout_pass::flow_axis(computed) == direction;
    let trim = crate::render::layout_pass::margin_trim::trimmed_edges(computed);
    // The queried axis runs from its physical end: a row under `rtl`, and
    // a reversed flex container's main axis (CSS Flexbox §5.1) — its
    // first item then sits at the right (bottom) edge.
    let flex_reversed =
        computed.flow == crate::layout::Flow::Flex && computed.flex_reverse && along;
    let reversed = match direction {
        Direction::Row => {
            crate::render::layout_pass::margin_trim::inline_reversed(computed) != flex_reversed
        }
        Direction::Column => flex_reversed,
    };
    let (trim_start, trim_end) = match direction {
        Direction::Row if reversed => (trim.right, trim.left),
        Direction::Row => (trim.left, trim.right),
        Direction::Column if reversed => (trim.bottom, trim.top),
        Direction::Column => (trim.top, trim.bottom),
    };
    let last = children.len() - 1;
    let flex = computed.flow == crate::layout::Flow::Flex;
    let outer = |i: usize, item: &FlexItem| {
        // Flexbox §4.4: a collapsed item is a strut — no main size, its
        // cross size kept.
        if flex && along && item.is_collapsed(dom) {
            return 0;
        }
        let keep_start = !(trim_start && (!along || i == 0));
        let keep_end = !(trim_end && (!along || i == last));
        let inner = match item {
            FlexItem::Element(c) => intrinsic_size_inner(
                dom,
                *c,
                direction,
                child_cross_budget,
                child_cb_width,
                IntrinsicMode::BoxSize,
                measure,
            ),
            // An anonymous item's box (a text run's has no declared size,
            // margins, padding or border; a pseudo-element's is its own).
            FlexItem::Anonymous(anon) => anon.box_size(
                dom,
                direction,
                child_cross_budget,
                child_cb_width,
                measure == Measure::MaxContent,
            ),
        };
        let cs = item.computed(dom);
        let (a, b) = match direction {
            Direction::Row if reversed => (&cs.margin.right, &cs.margin.left),
            Direction::Row => (&cs.margin.left, &cs.margin.right),
            Direction::Column if reversed => (&cs.margin.bottom, &cs.margin.top),
            Direction::Column => (&cs.margin.top, &cs.margin.bottom),
        };
        let side = |m: &crate::layout::MarginValue, keep: bool| {
            if keep {
                i32::from(m.resolve(child_cb_width))
            } else {
                0
            }
        };
        let margins = side(a, keep_start) + side(b, keep_end);
        (i32::from(inner) + margins).clamp(0, i32::from(u16::MAX)) as u16
    };
    // A multi-line flex container (CSS Flexbox §9.9): on its main axis
    // each item can take a line of its own, so its min-content size is
    // its largest item's (its max-content size is one line, the sum); on
    // its cross axis it is its lines'. A single-line row's height is its
    // one line's (§9.4 steps 8 / 15) — sized as a multi-line container's
    // line is, so its baseline-aligned extent counts. (A column's cross
    // axis is the inline axis, where `baseline` falls back and the
    // min-content contributions apply: its items' largest.)
    let wrapping = flex && crate::render::layout_pass::flex::is_multi_line(computed);
    let row_line = flex && !along && direction == Direction::Column;
    let intrinsic_children: u16 = if along && wrapping && measure == Measure::MinContent {
        children
            .iter()
            .enumerate()
            .map(|(i, c)| outer(i, c))
            .max()
            .unwrap_or(0)
    } else if (wrapping || row_line) && !along {
        wrap::wrapped_cross_size(
            dom,
            id,
            computed,
            children,
            direction,
            cross_budget,
            containing_block_width,
        )
    } else if along {
        // Children flow along the queried axis — sum their outer main
        // sizes plus gaps. Intrinsic sizing has no container size:
        // percent gaps are 0. A collapsed flex item takes no gap (CSS
        // Flexbox §9.4 step 10).
        // A block container's gaps sit between its block-level children
        // only — none around an inline child's anonymous block, as block
        // layout places them (`block::layout_block_children`).
        let spaced = children
            .iter()
            .filter(|c| match c {
                _ if flex => !c.is_collapsed(dom),
                FlexItem::Element(id) => dom
                    .node(*id)
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .is_none_or(|cs| cs.display == crate::layout::Display::Block),
                FlexItem::Anonymous(_) => false,
            })
            .count();
        let gap_total = crate::render::layout_pass::gap_along(computed, direction)
            .resolve(0)
            .saturating_mul((spaced as u16).saturating_sub(1));
        let children_main: u16 = children
            .iter()
            .enumerate()
            .map(|(i, c)| outer(i, c))
            .fold(0u16, |acc, n| acc.saturating_add(n));
        children_main.saturating_add(gap_total)
    } else {
        // Children stack across the queried axis — the largest outer size.
        children
            .iter()
            .enumerate()
            .map(|(i, c)| outer(i, c))
            .max()
            .unwrap_or(0)
    };

    // A block container's direct text runs next to element children
    // become anonymous block boxes (CSS 2.1 §9.2.1.1): a row each on the
    // Column axis (unwrapped estimate; block layout measures the real
    // wrap), the widest run on the Row axis. A flex container's are its
    // anonymous items, measured above.
    let text_runs = dom
        .node(id)
        .child_nodes()
        .filter(|c| {
            !flex
                && c.node_type() == NodeType::Text
                && c.node_value()
                    .is_some_and(|t| !t.chars().all(char::is_whitespace))
        })
        .map(|c| intrinsic_text(dom, c.id(), direction));
    let with_text = match direction {
        Direction::Column => text_runs.fold(intrinsic_children, |acc, n| acc.saturating_add(n)),
        Direction::Row => text_runs.fold(intrinsic_children, |acc, n| acc.max(n)),
    };
    // A `::before` / `::after` beside a block-level edge child is a line
    // box of its own (CSS 2.1 §9.2.1.1) — its rows add on the Column axis.
    match direction {
        Direction::Column => {
            with_text.saturating_add(own_line_pseudo_rows(dom, id, child_cross_budget))
        }
        Direction::Row => with_text,
    }
}
