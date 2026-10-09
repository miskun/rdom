//! Placing one block-level child in normal flow (CSS 2.1 §10.3.3 width,
//! §10.6.3 height, §8.3.1 margin collapsing at its edges, §9.5 / §9.5.2
//! the floats beside it) — shared by layout and measurement (`flow`).

use rdom_core::{Dom, NodeId};

use super::*;

/// Per-child placement context — bundles the in-flow positioning
/// state so the placement's signature stays narrow.
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

/// A block-level child where flow layout puts it, before it is laid out
/// or measured — the one placement layout and measurement share
/// (`flow`): its style, its border box (its height the pre-layout one,
/// `resolve_block_height`), where it was before the floats moved it, and
/// the margins and collapse-through state the cursor advances by.
pub(super) struct ChildPlaced {
    pub(super) computed: std::rc::Rc<ComputedStyle>,
    pub(super) rect: LayoutRect,
    /// Its position and width in flow, before the floats (§9.5.2, §9.5).
    flow: super::super::float::FlowBox,
    outer_bottom: MarginAccumulator,
    collapse_through: bool,
    cleared: bool,
    /// The margin-chain walks to memoize (`store_margin_chain_memo`).
    pub(super) memo: Vec<super::margin_collapse::ChainEntry>,
}

/// Place the block-level `child` at `ctx` among the floats of `area`:
/// fold its *outer* top margin into the running accumulator — the
/// collapse-eligible chain through its first block child and on down
/// (CSS 2.1 §8.3.1, `BFC1-MARGIN-COLLAPSE-UPWARD-1`), nothing when it
/// already escaped upward (`suppress_top_margin`) — resolve the
/// accumulator into the gap above it, and move it below the floats its
/// `clear` names (§9.5.2) or, a box that establishes a formatting context
/// of its own, beside or below the floats (§9.5). Its outer bottom margin
/// (through its last block child) waits for [`advance`]. An empty
/// collapse-through block (Phase 5.3) is placed at the resolved position
/// but advances nothing.
pub(super) fn place_block_child(
    dom: &Dom<TuiExt>,
    area: &crate::render::layout_pass::float::ExclusionArea,
    child: NodeId,
    ctx: &mut BlockPlace<'_>,
) -> ChildPlaced {
    let cb = ctx.containing_block_width;
    let computed = dom
        .node(child)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
    let resolved = resolve_block_width(dom, child, &computed, (cb, Some(ctx.container.height)));
    let height = resolve_block_height(
        dom,
        child,
        &computed,
        resolved.width,
        ctx.container.height,
        cb,
    );
    let mut memo = Vec::new();
    if !ctx.suppress_top_margin {
        ctx.margin_acc
            .merge(outer_top_margin(dom, child, &computed, cb, &mut memo));
    }
    let mut outer_bottom = MarginAccumulator::new();
    if !ctx.suppress_bottom_margin {
        outer_bottom = outer_bottom_margin(dom, child, &computed, cb, &mut memo);
    }
    let collapse_through = is_empty_collapse_through(dom, child, &computed, height);
    let flow = super::super::float::FlowBox {
        x0: ctx.container.x,
        cb_width: cb,
        x: ctx.container.x + resolved.margin_left as i32,
        y: ctx.y_cursor + i32::from(ctx.margin_acc.resolved()),
        width: resolved.width,
        rows: height,
    };
    let beside = super::super::float::beside_floats_in(dom, area, child, &computed, flow);
    ChildPlaced {
        rect: LayoutRect::new(beside.x, beside.y, beside.width, height),
        cleared: beside.y != flow.y,
        computed,
        flow,
        outer_bottom,
        collapse_through,
        memo,
    }
}

/// The block-level `child` placed alone at `(x, y)` in a containing block
/// `width` cells wide (no margins collapse with it): its border box — the
/// pre-layout height — below its top margin, and its bottom margin. A
/// multi-column container's spanner (CSS Multi-column 1 §6) is placed so.
pub(in crate::render::layout_pass) fn place_alone(
    dom: &Dom<TuiExt>,
    child: NodeId,
    x: i32,
    y: i32,
    width: u16,
) -> (LayoutRect, i32) {
    let computed = dom
        .node(child)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
    let resolved = resolve_block_width(dom, child, &computed, (width, None));
    let height = resolve_block_height(dom, child, &computed, resolved.width, 0, width);
    let top = i32::from(computed.margin.top.resolve(width));
    let bottom = i32::from(computed.margin.bottom.resolve(width));
    let rect = LayoutRect::new(
        x + i32::from(resolved.margin_left),
        y + top,
        resolved.width,
        height,
    );
    (rect, bottom)
}

/// CSS 2.1 §9.5: a box that establishes a formatting context of its own
/// "must not overlap the margin box of any floats" — at the height it
/// gets. Placed at its pre-layout height, a root beside floats that is
/// `height` rows once laid out (its text wrapping in the narrower band)
/// is placed again at that height: the border box it must move to, or
/// `None` where it stays. Once: the box laid out again may change height
/// again, which the caller does not chase (DIVERGENCES §2).
pub(super) fn replace_beside_floats(
    dom: &Dom<TuiExt>,
    area: &crate::render::layout_pass::float::ExclusionArea,
    child: NodeId,
    placed: &ChildPlaced,
    height: u16,
) -> Option<LayoutRect> {
    if height <= placed.rect.height
        || area.is_empty()
        || !establishes_bfc(dom, child, &placed.computed)
    {
        return None;
    }
    let flow = super::super::float::FlowBox {
        rows: height,
        ..placed.flow
    };
    let again = super::super::float::beside_floats_in(dom, area, child, &placed.computed, flow);
    let rect = LayoutRect::new(again.x, again.y, again.width, height);
    (rect.x != placed.rect.x || rect.y != placed.rect.y || rect.width != placed.rect.width)
        .then_some(rect)
}

/// Advance the flow past the child `placed`, `height` rows tall at
/// `top` (its border box's): the cursor after it, its outer bottom
/// margin buffered in `margin_acc` for the next sibling — or, an empty
/// collapse-through block not moved by clearance, the cursor where it was
/// and its outer bottom margin folded into the same accumulator, so the
/// next sibling's gap collapses with all of them.
pub(super) fn advance(
    placed: &ChildPlaced,
    top: i32,
    height: u16,
    y_cursor: i32,
    margin_acc: &mut MarginAccumulator,
) -> i32 {
    if placed.collapse_through && !placed.cleared {
        margin_acc.merge(placed.outer_bottom);
        y_cursor
    } else {
        *margin_acc = placed.outer_bottom;
        top + i32::from(height)
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
