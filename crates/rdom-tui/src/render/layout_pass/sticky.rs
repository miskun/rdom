//! Sticky positioning placement (M5.4).
//!
//! `position: sticky` elements stay in normal flow during the main
//! flex / inline pass. This pass runs after that pass completes and
//! adjusts each sticky element's `LayoutRect` based on the nearest
//! scrollable ancestor (the "scrollport") and the threshold insets
//! (`top`, `left`).
//!
//! Three-phase model (CSS Positioned Layout L3):
//!
//! 1. **Pre-stick** — sticky's normal-flow rect is above the
//!    threshold; element scrolls with the page as usual.
//! 2. **Stuck** — sticky's normal-flow rect would scroll past the
//!    threshold; the rect pins to the threshold edge instead.
//! 3. **Post-stick** — the containing block's far edge has also
//!    scrolled past; sticky is clamped so it can't extend beyond
//!    the containing block, and effectively scrolls out with it.
//!
//! v1 surface: vertical `top` insets and horizontal `left` insets.
//! `right` / `bottom` sticky directions and full CSS scrollport-vs-
//! containing-block separation are deferred polish.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Length, Overflow, Position};
use crate::node::TuiNodeExt;

/// Walk the tree, find every sticky element, and rewrite its
/// `LayoutRect` to pin against its scrollport when the scroll
/// position requires it.
pub(super) fn place_sticky(dom: &mut Dom<TuiExt>) {
    let sticky_ids = collect_sticky(dom, dom.root());
    for id in sticky_ids {
        place_one(dom, id);
    }
}

fn collect_sticky(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, id, &mut out);
    out
}

fn walk(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        let pos = dom
            .node(id)
            .computed()
            .map(|c| c.position)
            .unwrap_or(Position::Static);
        if pos == Position::Sticky {
            out.push(id);
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => walk(dom, child.id(), out),
            _ => {}
        }
    }
}

fn place_one(dom: &mut Dom<TuiExt>, id: NodeId) {
    // Snapshot what we need before mutating.
    let Some(natural) = dom.node(id).ext().map(|e| e.layout) else {
        return;
    };
    let computed = match dom.node(id).computed().cloned() {
        Some(c) => c,
        None => return,
    };
    let (top_inset, left_inset) = (computed.top, computed.left);

    // Find nearest scrollable ancestor. Scrollable = overflow_x or
    // overflow_y not Visible.
    let scrollport = nearest_scrollport(dom, id);
    let Some((scrollport_id, scrollport_rect)) = scrollport else {
        // CSS rule: no scrollable ancestor → sticky behaves as
        // relative (position-as-laid-out, no pin). Nothing to do.
        return;
    };

    // Containing block (post-stick clamp): parent element's
    // `content_layout`. CSS uses the sticky's containing block,
    // which for v1 we approximate by the parent.
    let cb_rect = dom
        .node(id)
        .parent_node()
        .and_then(|p| p.ext().map(|e| e.content_layout))
        .unwrap_or(scrollport_rect);

    let mut placed = natural;

    // Vertical sticky with `top: N` (cells).
    if let Length::Cells(n) = top_inset {
        let pin_y = scrollport_rect.y.saturating_add(n as i32);
        if placed.y < pin_y {
            // Stuck — pin to threshold.
            placed.y = pin_y;
        }
        // Post-stick clamp: don't extend beyond the containing block.
        let cb_far = cb_rect.bottom().saturating_sub(placed.height as i32);
        if placed.y > cb_far {
            placed.y = cb_far;
        }
        // And don't move the element OFF the containing block start
        // — pre-stick stays at its natural position when it hasn't
        // crossed the threshold yet.
        if placed.y < natural.y && natural.y < pin_y {
            // Still pre-stick relative to natural y but our clamp
            // pulled us back; restore. (Defensive — this branch is
            // unreachable given the order above, but documents intent.)
            placed.y = natural.y;
        }
    }
    // Horizontal sticky with `left: N` (cells).
    if let Length::Cells(n) = left_inset {
        let pin_x = scrollport_rect.x.saturating_add(n as i32);
        if placed.x < pin_x {
            placed.x = pin_x;
        }
        let cb_far = cb_rect.right().saturating_sub(placed.width as i32);
        if placed.x > cb_far {
            placed.x = cb_far;
        }
    }

    if placed == natural {
        // No-op — element is in its pre-stick phase.
        return;
    }

    // Recursively shift the sticky's subtree by the delta. CSS:
    // sticky's children move with it (matches `position: relative`
    // behavior).
    let dx = placed.x - natural.x;
    let dy = placed.y - natural.y;
    let _ = scrollport_id; // reserved for future debug logging
    shift_subtree(dom, id, dx, dy);
}

fn shift_subtree(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout = LayoutRect::new(
            ext.layout.x + dx,
            ext.layout.y + dy,
            ext.layout.width,
            ext.layout.height,
        );
        ext.content_layout = LayoutRect::new(
            ext.content_layout.x + dx,
            ext.content_layout.y + dy,
            ext.content_layout.width,
            ext.content_layout.height,
        );
    }
    let child_ids: Vec<NodeId> = dom
        .node(id)
        .child_nodes()
        .filter(|c| {
            matches!(
                c.node_type(),
                NodeType::Element | NodeType::Fragment | NodeType::Text
            )
        })
        .map(|c| c.id())
        .collect();
    for c in child_ids {
        shift_subtree(dom, c, dx, dy);
    }
}

fn nearest_scrollport(dom: &Dom<TuiExt>, id: NodeId) -> Option<(NodeId, LayoutRect)> {
    let mut cursor = dom.node(id).parent_node();
    while let Some(p) = cursor {
        if p.node_type() == NodeType::Element {
            let computed = p.computed();
            let scrollable = computed
                .map(|c| c.overflow_x != Overflow::Visible || c.overflow_y != Overflow::Visible)
                .unwrap_or(false);
            if scrollable && let Some(ext) = p.ext() {
                // CSS Overflow 3 §3 + Position 3 sticky: pin against
                // the scrollport (= padding-box), not `content_layout`.
                // Under M5.5b border-collapse `content_layout` can
                // widen into the border ring; using it here would
                // shift the sticky pin threshold 1 row earlier on each
                // expanded edge.
                let border = computed.map(|c| c.border).unwrap_or_default();
                let scrollport = rdom_style::layout::compute_padding_box(ext.layout, border);
                return Some((p.id(), scrollport));
            }
        }
        cursor = p.parent_node();
    }
    None
}
