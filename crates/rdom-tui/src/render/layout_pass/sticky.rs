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
use crate::layout::{LayoutRect, Length, Position};
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
            .filter(|c| c.display != crate::layout::Display::Contents)
            .map(|c| c.position)
            .unwrap_or(Position::Static);
        if pos == Position::Sticky {
            out.push(id);
        }
    }
    for child in crate::render::box_tree::children(dom, id) {
        match dom.node(child).node_type() {
            NodeType::Element | NodeType::Fragment => walk(dom, child, out),
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

    // Find the nearest scroll container ancestor.
    let parent = crate::render::box_tree::slot::parent(dom, id);
    let Some(scrollport) = nearest_scrollport(dom, parent) else {
        // CSS rule: no scrollable ancestor → sticky behaves as
        // relative (position-as-laid-out, no pin). Nothing to do.
        return;
    };

    // Containing block (post-stick clamp): parent element's
    // `content_layout`. CSS uses the sticky's containing block,
    // which for v1 we approximate by the parent.
    let cb_rect = parent
        .and_then(|p| dom.node(p).ext().map(|e| e.content_layout))
        .unwrap_or(scrollport);

    // Stickiness is the laid-out box's, untransformed (CSS Position 3
    // §3.4, Transforms 1 §3): its translation, already applied to
    // `layout`, stays on top of the stick (C15G-TRANSLATE-GAPS).
    let (tx, ty) = dom.node(id).ext().map_or((0, 0), |e| {
        crate::style::effects::translation(&computed, e.layout, e.content_layout)
    });
    let natural = LayoutRect {
        x: natural.x - tx,
        y: natural.y - ty,
        ..natural
    };
    let (dx, dy) = sticky_offset(&computed, natural, scrollport, cb_rect);
    if (dx, dy) == (0, 0) {
        // No-op — element is in its pre-stick phase.
        return;
    }
    // Recursively shift the sticky's subtree by the delta. CSS:
    // sticky's children move with it (matches `position: relative`
    // behavior), and so do the absolutely positioned boxes it contains
    // (CSS Position 3 §2.1); a `fixed` descendant stays on the viewport.
    super::tree::shift_subtree(dom, id, dx, dy);
}

/// The `(dx, dy)` a sticky box styled `computed`, laid out in flow at
/// `natural`, moves by (CSS Position 3 §3.4): pinned inside `scrollport`
/// by its insets once scrolling would carry it past them, never out of
/// its containing block `cb_rect`. Shared by sticky elements and sticky
/// pseudo-elements.
pub(super) fn sticky_offset(
    computed: &crate::style::ComputedStyle,
    natural: LayoutRect,
    scrollport_rect: LayoutRect,
    cb_rect: LayoutRect,
) -> (i32, i32) {
    let mut placed = natural;

    // Insets resolve like CSS Position 3 §3.4: percentages / `calc()`
    // against the scrollport's size on that axis; `auto` means "no
    // constraint on this edge".
    let inset = |len: &Length, basis: u16| len.cells(i32::from(basis));
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
    (placed.x - natural.x, placed.y - natural.y)
}

/// The scrollport of the nearest scroll container at or above `from`
/// (CSS Position 3 §3.4) — a sticky element's from its parent up, a
/// sticky pseudo-element's from its host up.
pub(super) fn nearest_scrollport(dom: &Dom<TuiExt>, from: Option<NodeId>) -> Option<LayoutRect> {
    let mut cursor = from.map(|id| dom.node(id));
    while let Some(p) = cursor {
        if p.node_type() == NodeType::Element {
            let computed = p.computed();
            // A scroll container (CSS Position 3 §3.4: the nearest
            // scrollport); an `overflow: clip` box is not one.
            let scrollable = computed.is_some_and(|c| c.is_scroll_container());
            if scrollable
                && let Some(ext) = p.ext()
                && let Some(c) = computed
            {
                // CSS Position 3 §3.4: pin against the scrollport — the
                // padding box less the scrollbar gutters (`scrollport`),
                // not `content_layout`, which under M5.5b border-collapse
                // can widen into the border ring.
                return Some(crate::render::layout_pass::scrollport_of(ext, c));
            }
        }
        cursor = crate::render::box_tree::slot::parent(dom, p.id()).map(|n| dom.node(n));
    }
    None
}
