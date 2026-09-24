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
//! All four insets (`top` / `bottom` / `left` / `right`, cells or
//! `calc()` against the scrollport) pin against the nearest scrollport;
//! the containing block is approximated by the parent's content box.

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

    // Insets resolve like CSS Position 3 §3.4: percentages / `calc()`
    // against the scrollport's size on that axis; `auto` means "no
    // constraint on this edge".
    let inset = |len: &Length, basis: u16| -> Option<i32> {
        match len {
            Length::Auto => None,
            Length::Cells(n) => Some(*n),
            Length::Calc(expr) => {
                Some(expr.resolve(&rdom_style::calc::ResolveCtx::new(i32::from(basis))))
            }
        }
    };
    let top_inset = inset(&computed.top, scrollport_rect.height);
    let bottom_inset = inset(&computed.bottom, scrollport_rect.height);
    let left_inset = inset(&computed.left, scrollport_rect.width);
    let right_inset = inset(&computed.right, scrollport_rect.width);

    // Vertical sticky with `top: N`.
    if let Some(n) = top_inset {
        let pin_y = scrollport_rect.y.saturating_add(n);
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
    // `bottom: N` — pin when the box would scroll past the scrollport's
    // bottom edge; never above the containing block's start
    // (`M5-STICKY-1`).
    if let Some(n) = bottom_inset {
        let pin_bottom = scrollport_rect.bottom().saturating_sub(n);
        if placed.bottom() > pin_bottom {
            placed.y = pin_bottom.saturating_sub(placed.height as i32);
        }
        if placed.y < cb_rect.y {
            placed.y = cb_rect.y;
        }
    }
    // Horizontal sticky with `left: N`.
    if let Some(n) = left_inset {
        let pin_x = scrollport_rect.x.saturating_add(n);
        if placed.x < pin_x {
            placed.x = pin_x;
        }
        let cb_far = cb_rect.right().saturating_sub(placed.width as i32);
        if placed.x > cb_far {
            placed.x = cb_far;
        }
    }
    // `right: N`, mirror of `bottom`.
    if let Some(n) = right_inset {
        let pin_right = scrollport_rect.right().saturating_sub(n);
        if placed.right() > pin_right {
            placed.x = pin_right.saturating_sub(placed.width as i32);
        }
        if placed.x < cb_rect.x {
            placed.x = cb_rect.x;
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
        // The element's own text lives in anonymous block boxes (mixed
        // content) and its pseudos in `before_layout` / `after_layout`;
        // both carry their own rects and pin with the element.
        for anon in &mut ext.anonymous_blocks {
            anon.rect = LayoutRect::new(
                anon.rect.x + dx,
                anon.rect.y + dy,
                anon.rect.width,
                anon.rect.height,
            );
        }
        for pseudo in [&mut ext.before_layout, &mut ext.after_layout]
            .into_iter()
            .flatten()
        {
            pseudo.rect = LayoutRect::new(
                pseudo.rect.x + dx,
                pseudo.rect.y + dy,
                pseudo.rect.width,
                pseudo.rect.height,
            );
        }
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
