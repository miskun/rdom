//! Placing one block-level child in normal flow (CSS 2.1 §10.3.3 width,
//! §10.6.3 height, §8.3.1 margin collapsing at its edges).

use rdom_core::{Dom, NodeId};

use super::*;

/// Per-child placement context — bundles the in-flow positioning
/// state so `lay_out_block_child`'s signature stays narrow.
pub(super) struct BlockPlace<'a> {
    pub(super) container: LayoutRect,
    pub(super) containing_block_width: u16,
    pub(super) y_cursor: i32,
    pub(super) margin_acc: &'a mut MarginAccumulator,
    /// Phase 5.2 — first-block-in-this-container + parent's
    /// `parent_collapses_top_with_first_child`. When true, the
    /// child's `margin-top` is suppressed to model parent-first-
    /// child collapse.
    pub(super) suppress_top_margin: bool,
    /// Symmetric to `suppress_top_margin` — for the last in-flow
    /// block child + parent's `parent_collapses_bottom_with_last_child`.
    pub(super) suppress_bottom_margin: bool,
}

/// Lay out a single block-level child. Folds the child's `margin-top`
/// into the running margin accumulator, resolves the accumulator into
/// a single gap above the child, places the child, then primes the
/// accumulator with the child's `margin-bottom` for the next sibling.
///
/// Returns the new y cursor — the bottom edge of the child's outer
/// rect (NOT including its bottom margin, which is now buffered in
/// `margin_acc`). The container's own height computation and the
/// parent-last-child collapse consume the leftover accumulator
/// separately.
pub(super) fn lay_out_block_child(
    dom: &mut Dom<TuiExt>,
    child: NodeId,
    ctx: BlockPlace<'_>,
) -> i32 {
    let BlockPlace {
        container,
        containing_block_width,
        y_cursor,
        margin_acc,
        suppress_top_margin,
        suppress_bottom_margin,
    } = ctx;
    let computed = dom
        .node(child)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

    let resolved = resolve_block_width(dom, child, &computed, containing_block_width);
    let height = resolve_block_height(
        dom,
        child,
        &computed,
        resolved.width,
        container.height,
        containing_block_width,
    );

    // Phase 5.2 + 5.4 — fold this child's *outer top* margin into
    // the accumulator. `accumulate_outer_top_margin` walks the
    // collapse-eligible chain (this child, its first block child
    // if they collapse, that one's first block child, …) so the
    // grandparent / great-grandparent sees the merged margin
    // surfacing at this block's outer top edge per CSS 2.1 §8.3.1.
    // Closes `BFC1-MARGIN-COLLAPSE-UPWARD-1`.
    //
    // When `suppress_top_margin` is set, this child's top margin
    // already escaped upward via the parent's call to this function
    // — contribute nothing here.
    let mut memo = Vec::new();
    if !suppress_top_margin {
        margin_acc.merge(outer_top_margin(
            dom,
            child,
            &computed,
            containing_block_width,
            &mut memo,
        ));
    }

    // Symmetric: compute the outer bottom margin (chain through
    // last collapse-eligible block descendant) so the NEXT sibling
    // sees the merged value, not just the raw `margin-bottom`. When
    // `suppress_bottom_margin` is set, the bottom already escaped
    // upward (parent-last-child collapse).
    let mut outer_bottom = MarginAccumulator::new();
    if !suppress_bottom_margin {
        outer_bottom =
            outer_bottom_margin(dom, child, &computed, containing_block_width, &mut memo);
    }
    store_margin_chain_memo(dom, &memo);

    // Phase 5.3 — empty-block collapse-through. A block with no
    // content, no padding, no border, and zero height has its top
    // + bottom margins meet — they fold into the surrounding
    // accumulator together rather than resetting it.
    let collapse_through = is_empty_collapse_through(dom, child, &computed, height);

    let outer_x = container.x + resolved.margin_left as i32;
    // The "gap" used for placement: for collapse-through children we
    // still place the empty box visually at the resolved-so-far
    // position (mostly for downstream layouts that ask for its
    // rect), but we do NOT advance the y_cursor or reset the
    // accumulator — the next sibling's gap will collapse with
    // everything accumulated so far.
    let gap = margin_acc.resolved();
    let outer_y = y_cursor + gap as i32;
    // The floats of the formatting context: clearance (CSS 2.1 §9.5.2)
    // moves the box below the floats its `clear` names, and a box that
    // establishes a formatting context of its own goes beside the floats
    // or below them, never over them (§9.5).
    let beside = super::super::float::beside_floats(
        dom,
        child,
        &computed,
        super::super::float::FlowBox {
            x0: container.x,
            cb_width: containing_block_width,
            x: outer_x,
            y: outer_y,
            width: resolved.width,
            rows: height,
        },
    );
    let cleared = beside.y != outer_y;
    let (outer_x, outer_y) = (beside.x, beside.y);
    let outer_rect = LayoutRect::new(outer_x, outer_y, beside.width, height);
    layout_node(dom, child, outer_rect, containing_block_width);

    // `layout_node` finalizes an `Auto` height via CSS 2.1 §10.6.3
    // content measurement, which can exceed the pre-layout
    // `resolve_block_height` estimate — notably for a mixed-content
    // block (a text run + a block child), whose `intrinsic_size`
    // walk counts element children only and so misses the text
    // run's anonymous-block row. Advance the cursor by the child's
    // ACTUAL laid-out height so the next sibling can't overlap it.
    // Use the intended `outer_y` (not the written `rect.y`, which
    // may carry a `position: relative` shift that must NOT move
    // siblings).
    let actual_height = dom
        .node(child)
        .layout_rect()
        .map(|r| r.height)
        .unwrap_or(height);

    if collapse_through && !cleared {
        // Fold the outer bottom into the SAME accumulator and
        // leave y_cursor where it was. Next sibling's `gap`
        // computation will see all of A.mb, E.mt, E.mb, B.mt.
        margin_acc.merge(outer_bottom);
        y_cursor
    } else {
        // Normal block: advance y_cursor past the child and reset
        // the accumulator to just this child's outer bottom margin.
        *margin_acc = outer_bottom;
        outer_y + actual_height as i32
    }
}

/// BORDER-MODEL-1 (M6) block-flow sibling overlap: under a `border-collapse:
/// collapse` parent, a block whose top border meets its previous block
/// sibling's bottom border shares that row (the cursor pulls back one, and
/// paint's mask-OR draws the junction), as `flex.rs` does. Any non-`none`
/// border counts, `hidden` included — it suppresses paint at the shared
/// cell but still takes part so the shared cell exists for the
/// kill-switch to suppress (`has_effective_border_on_edge`).
pub(super) fn borders_overlap(
    dom: &Dom<TuiExt>,
    parent: &ComputedStyle,
    prev: NodeId,
    child: NodeId,
) -> bool {
    if parent.border_collapse != crate::layout::BorderCollapse::Collapse {
        return false;
    }
    let prev_bottom = dom
        .node(prev)
        .computed()
        .is_some_and(|c| !c.border.bottom.is_none());
    let top = dom
        .node(child)
        .computed()
        .is_some_and(|c| !c.border.top.is_none());
    prev_bottom && top
}
