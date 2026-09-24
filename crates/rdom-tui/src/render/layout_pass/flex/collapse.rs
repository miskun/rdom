//! Flex-specific `border-collapse: collapse` rules.
//!
//! Two places in a flex line depend on the table-cell border model
//! and must agree with each other:
//!
//! - the **parent-edge inset** applied to the container before any
//!   distribution, so a borderless first / last child does not land
//!   on the parent's painted border row; and
//! - the **one-cell sibling overlap** between adjacent bordered items,
//!   which the sizing budget reclaims up front and the placement
//!   cursor pulls back per pair (BORDER-MODEL-1 gap-honoring rule).
//!
//! The per-edge border predicates themselves live in
//! [`super::super::border_collapse`]; this module only decides when
//! and where they apply along a flex line.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{BorderCollapse, Direction, LayoutRect};
use crate::render::layout_pass::border_collapse::{
    CollapseEdge, collapse_parent_edge_insets, has_effective_border_on_edge,
};
use crate::style::ComputedStyle;

/// Shrink `container` by the parent-child border-collapse inset.
///
/// Under `border-collapse: collapse`, `compute_content_area_collapsed`
/// flattens the parent's content area to its outer rect — children's
/// outer rects then extend into the parent's border ring (so a
/// bordered child's first cell coincides with the parent's first
/// border cell, sharing one paint surface — the table-cell model).
///
/// That sharing is only correct when the first/last child ACTUALLY
/// HAS A BORDER to share. If the first child is content-bearing
/// (no own border), its content would land on the parent's painted
/// border row and disappear under the border glyph. Surfaced
/// visually by the showcase chrome: `<header>` inside an `<app>`
/// with collapse + own border had its `<h1>` text painted at the
/// shared border row.
///
/// Per-edge fix: if the first child along the main axis has no
/// border, push that edge's start back by 1 so the first child's
/// content area sits below the parent's border row. Same for the
/// last child along the main axis. Cross-axis insets follow the
/// same logic. Pre-scan one element child each direction; correct
/// for the common case (table cells vs. content-bearing chrome
/// panels) without touching `compute_content_area_collapsed`.
pub(super) fn inset_container_for_children(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
    parent: &ComputedStyle,
    container: LayoutRect,
) -> LayoutRect {
    let (top_inset, bot_inset, left_inset, right_inset) =
        collapse_parent_edge_insets(dom, children, parent);
    LayoutRect::new(
        container.x + left_inset as i32,
        container.y + top_inset as i32,
        container.width.saturating_sub(left_inset + right_inset),
        container.height.saturating_sub(top_inset + bot_inset),
    )
}

/// Whether adjacent bordered siblings on this line share one cell on
/// their meeting edge, and which edges meet.
///
/// Under `border-collapse: collapse`, each pair of adjacent
/// bordered siblings shares one cell on their meeting edge. The
/// cursor advance subtracts 1 per overlap (see the placement
/// loop), but flex sizing needs to know up-front so the
/// grow distribution uses ALL the available cells — otherwise
/// the saved cells appear as empty space at the parent's right
/// / bottom edge.
///
/// **BORDER-MODEL-1 gap-honoring rule.** When the author writes
/// `gap > 0`, the gap is sacred: it produces a visible cell
/// between siblings and there is nothing adjacent to merge. So
/// the overlap only fires when `gap == 0` AND the parent
/// declares `collapse`. With `gap > 0`, the gap and collapse
/// coexist orthogonally — `collapse` becomes a no-op for that
/// sibling pair. Documented as the 2×2 outcome grid in
/// `DIVERGENCES.md`.
pub(super) struct SiblingOverlap {
    /// `gap == 0 && parent.border_collapse == Collapse`.
    active: bool,
    /// The trailing edge of item `i` along the main axis.
    edge_i: CollapseEdge,
    /// The leading edge of item `i + 1` along the main axis.
    edge_next: CollapseEdge,
}

impl SiblingOverlap {
    pub(super) fn new(parent: &ComputedStyle, gap: u16, direction: Direction) -> Self {
        let (edge_i, edge_next) = match direction {
            Direction::Column => (CollapseEdge::Bottom, CollapseEdge::Top),
            Direction::Row => (CollapseEdge::Right, CollapseEdge::Left),
        };
        Self {
            active: parent.border_collapse == BorderCollapse::Collapse && gap == 0,
            edge_i,
            edge_next,
        }
    }

    /// Whether items `a` (earlier) and `b` (its next sibling) share a
    /// cell: only when overlap is active AND both have a border on
    /// the shared edge.
    pub(super) fn between(&self, dom: &Dom<TuiExt>, a: NodeId, b: NodeId) -> bool {
        self.active
            && has_effective_border_on_edge(dom, a, self.edge_i)
            && has_effective_border_on_edge(dom, b, self.edge_next)
    }

    /// Total cells reclaimed by sibling overlap across the line — one
    /// per adjacent pair that [`Self::between`] accepts.
    pub(super) fn savings(&self, dom: &Dom<TuiExt>, children: &[NodeId]) -> u16 {
        if !self.active {
            return 0;
        }
        let mut savings: u16 = 0;
        for i in 0..children.len().saturating_sub(1) {
            if self.between(dom, children[i], children[i + 1]) {
                savings = savings.saturating_add(1);
            }
        }
        savings
    }
}
