//! Inline-flow target selection — which IFC (or anonymous block box)
//! a text position should be resolved in.
//!
//! Owns [`InlineTarget`], the containment lookup `inline_target_at`
//! used by [`HitTestExt::position_at`](super::HitTestExt::position_at),
//! and the empty-space fallback [`nearest_inline_target_in_subtree`]
//! (nearest by vertical distance, skipping `user-select: none`).
//! Turning a target plus `(x, y)` into a [`rdom_core::Position`] is
//! `fragment.rs`.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::inline::has_inline_layout;
use crate::runtime::selection::user_select;

/// The inline-flow target in `root`'s subtree nearest to `y` by vertical
/// distance — the empty-space fallback for [`HitTestExt::position_at`]. A
/// point inside a target's y-range has distance 0; otherwise it's the gap
/// to the nearest edge. Ties keep the first found in document order.
/// `user-select: none` candidates are skipped so the snap never lands on
/// unselectable chrome. Returns `None` when the subtree has no inline flow.
///
/// [`HitTestExt::position_at`]: super::HitTestExt::position_at
pub(crate) fn nearest_inline_target_in_subtree(
    dom: &Dom<TuiExt>,
    root: NodeId,
    y: u16,
) -> Option<InlineTarget> {
    let y = y as i32;
    let mut best: Option<(i32, InlineTarget)> = None;
    let mut consider = |target: InlineTarget, top: i32, bottom: i32| {
        if user_select::is_unselectable(dom, target.node()) {
            return;
        }
        let dist = if y < top {
            top - y
        } else if y >= bottom {
            y - bottom + 1
        } else {
            0
        };
        if best.is_none_or(|(d, _)| dist < d) {
            best = Some((dist, target));
        }
    };

    // Document-order DFS so ties resolve to the earliest target.
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if has_inline_layout(dom, id)
            && let Some(content) = crate::render::inline::scrolled_content_rect(dom, id)
        {
            consider(
                InlineTarget::Ifc(id),
                content.y,
                content.y + content.height as i32,
            );
        }
        if let Some(ext) = dom.node(id).ext() {
            for (i, anon) in ext.anonymous_blocks.iter().enumerate() {
                consider(
                    InlineTarget::Anonymous {
                        container: id,
                        index: i,
                    },
                    anon.rect.y,
                    anon.rect.y + anon.rect.height as i32,
                );
            }
        }
        // Push children reversed so they pop in document order.
        let kids: Vec<NodeId> = crate::render::box_tree::children(dom, id)
            .filter(|&c| dom.node(c).node_type() == rdom_core::NodeType::Element)
            .collect();
        stack.extend(kids.into_iter().rev());
    }
    best.map(|(_, t)| t)
}

/// What kind of inline-flow container is under the hit point.
#[derive(Debug, Clone, Copy)]
pub(crate) enum InlineTarget {
    /// Classic IFC — the block element itself owns the
    /// `inline_layout`. Content rect = the block's content_layout.
    Ifc(NodeId),
    /// Anonymous block box — `container` owns the
    /// `anonymous_blocks` Vec; `index` selects the entry. Content
    /// rect = the entry's `.rect` (no further inset).
    Anonymous { container: NodeId, index: usize },
}

impl InlineTarget {
    /// The owning node — the IFC block itself, or the container that
    /// holds the anonymous block box. Used for the `user-select` gate
    /// on the nearest-flow fallback.
    fn node(self) -> NodeId {
        match self {
            InlineTarget::Ifc(id) => id,
            InlineTarget::Anonymous { container, .. } => container,
        }
    }

    /// Resolve to `(layout, content_rect)`. Borrows from the dom.
    pub(super) fn layout_and_rect(
        self,
        dom: &Dom<TuiExt>,
    ) -> Option<(&crate::render::inline::InlineLayout, LayoutRect)> {
        match self {
            InlineTarget::Ifc(id) => {
                let ext = dom.node(id).ext()?;
                let layout = ext.inline_layout.as_ref()?;
                let content = crate::render::inline::scrolled_content_rect(dom, id)?;
                Some((layout, content))
            }
            InlineTarget::Anonymous { container, index } => {
                let ext = dom.node(container).ext()?;
                let anon = ext.anonymous_blocks.get(index)?;
                Some((&anon.inline_layout, anon.rect))
            }
        }
    }
}

/// Return the inline-flow target rooted at `id` that contains the
/// point, if any. Picks the singular IFC when present; otherwise the
/// anonymous box holding the point — of several on its rows (a flex
/// container's anonymous items share a line, CSS Flexbox §4), the one
/// whose columns hold `x`, else the first on the rows (a block
/// container's span its content width).
pub(super) fn inline_target_at(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
) -> Option<InlineTarget> {
    if has_inline_layout(dom, id) {
        return Some(InlineTarget::Ifc(id));
    }
    let ext = dom.node(id).ext()?;
    let (x, y) = (i32::from(x), i32::from(y));
    let on_rows = |anon: &&crate::ext::AnonymousIfc| {
        y >= anon.rect.y && y < anon.rect.y + i32::from(anon.rect.height)
    };
    let in_columns = |anon: &crate::ext::AnonymousIfc| {
        x >= anon.rect.x && x < anon.rect.x + i32::from(anon.rect.width)
    };
    let index = ext
        .anonymous_blocks
        .iter()
        .position(|anon| on_rows(&anon) && in_columns(anon))
        .or_else(|| ext.anonymous_blocks.iter().position(|anon| on_rows(&anon)))?;
    Some(InlineTarget::Anonymous {
        container: id,
        index,
    })
}
