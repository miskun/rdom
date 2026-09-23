//! `border-collapse: collapse` edge sharing — the helpers both the block
//! and the flex pass consult when deciding whether a child's outer edge
//! shares the parent's border row (BORDER-MODEL-1). Lifted out of
//! `flex.rs` (BFC1-CODE-COLLAPSE-INSETS-1) so neither pass owns what
//! both use.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

/// Edge tag used for transparent-intermediate border-collapse
/// propagation. Independent of the `Border` enum so we can talk
/// about a single edge without conflating it with the four
/// "single-edge-only" `Border` variants.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum CollapseEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Does `id` declare a border on `edge` for collapse-sharing
/// purposes? Returns `true` for any non-`None` style — including
/// `Hidden`. Hidden contributes to the cell's conflict resolution
/// (it's CSS Tables 3 §11.5 rule 1's kill-switch) AND it
/// participates in layout-side sibling-overlap so the shared
/// cell exists for the kill-switch to suppress. Only `None`
/// fully opts out.
///
/// **BORDER-MODEL-1 simplification.** The previous version recursed
/// through borderless transparent intermediates to find bordered
/// descendants — a heuristic stack that needed escape hatches for
/// padding, margin, declared-collapse, etc. Under the new model
/// (`border-collapse: collapse` is non-inheriting and affects a
/// container's direct children only), collapse-sharing is a
/// per-direct-child decision. Each child reports on its OWN
/// border; the recursion is unnecessary and was actively wrong for
/// any child whose bordered descendants were offset from its edge
/// by padding, margin, or its own collapse declaration.
pub(super) fn has_effective_border_on_edge(
    dom: &Dom<TuiExt>,
    id: NodeId,
    edge: CollapseEdge,
) -> bool {
    // A single `bool` — borrow the computed style, never clone it (this
    // runs twice per sibling pair, twice, on every flex layout).
    let Some(computed) = dom.node(id).computed() else {
        return false; // initial style: no border
    };
    match edge {
        CollapseEdge::Top => !computed.border.top.is_none(),
        CollapseEdge::Bottom => !computed.border.bottom.is_none(),
        CollapseEdge::Left => !computed.border.left.is_none(),
        CollapseEdge::Right => !computed.border.right.is_none(),
    }
}

/// Per-edge inset to add back under `border-collapse: collapse` when
/// the first/last child along the main axis has no own border.
/// Returns `(top, bottom, left, right)` in cells. All zero unless
/// parent has both `collapse` and an own border AND a relevant
/// child lacks a border.
///
/// See the call site for full rationale. Short version: the flatten
/// in `compute_content_area_collapsed` is correct only when the
/// shared border row is actually shared with a child's own border;
/// when the child is content-bearing (no border), it would land
/// on the parent's painted border row.
pub(super) fn collapse_parent_edge_insets(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
    parent: &ComputedStyle,
) -> (u16, u16, u16, u16) {
    use crate::layout::BorderCollapse;
    if parent.border_collapse != BorderCollapse::Collapse {
        return (0, 0, 0, 0);
    }
    let parent_has_top = !parent.border.top.is_none();
    let parent_has_bottom = !parent.border.bottom.is_none();
    let parent_has_left = !parent.border.left.is_none();
    let parent_has_right = !parent.border.right.is_none();
    if !(parent_has_top || parent_has_bottom || parent_has_left || parent_has_right) {
        return (0, 0, 0, 0);
    }

    // Per-edge inset: only inset when parent has that edge AND
    // the child whose outer edge shares it lacks an effective
    // border on the same edge (direct OR via transparent
    // intermediate containers — same helper used by the sibling-
    // overlap path so both axes treat "transparency" consistently).
    let first = *children.first().unwrap();
    let last = *children.last().unwrap();
    let needs_inset =
        |id: NodeId, edge: CollapseEdge| -> bool { !has_effective_border_on_edge(dom, id, edge) };

    let (top, bottom, left, right) = match parent.direction {
        Direction::Column => {
            // Main axis vertical. First column-child touches
            // parent's top; last touches parent's bottom. Cross
            // axis (left/right) — first child stands in.
            let top = if parent_has_top && needs_inset(first, CollapseEdge::Top) {
                1
            } else {
                0
            };
            let bottom = if parent_has_bottom && needs_inset(last, CollapseEdge::Bottom) {
                1
            } else {
                0
            };
            let left = if parent_has_left && needs_inset(first, CollapseEdge::Left) {
                1
            } else {
                0
            };
            let right = if parent_has_right && needs_inset(first, CollapseEdge::Right) {
                1
            } else {
                0
            };
            (top, bottom, left, right)
        }
        Direction::Row => {
            // Mirror of column.
            let left = if parent_has_left && needs_inset(first, CollapseEdge::Left) {
                1
            } else {
                0
            };
            let right = if parent_has_right && needs_inset(last, CollapseEdge::Right) {
                1
            } else {
                0
            };
            let top = if parent_has_top && needs_inset(first, CollapseEdge::Top) {
                1
            } else {
                0
            };
            let bottom = if parent_has_bottom && needs_inset(first, CollapseEdge::Bottom) {
                1
            } else {
                0
            };
            (top, bottom, left, right)
        }
    };
    (top, bottom, left, right)
}
