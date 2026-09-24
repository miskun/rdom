//! Flex-child distribution — the core layout math.
//!
//! Given a container's `content_layout` and its direct element
//! children (plus the parent's `direction` + `gap`), computes each
//! child's main-axis + cross-axis size and recursively lays them out.
//!
//! Main-axis sizing in order of precedence:
//! 1. `Size::Fixed(n)` → exactly `n`.
//! 2. `Size::Percent(p)` → `main_budget * p / 100` (treated as
//!    fixed once resolved; does not participate in flex distribution).
//! 3. `Size::Auto` → intrinsic (content fit), via [`intrinsic::intrinsic_size`].
//! 4. `Size::Flex(w)` → share of the remaining main-axis budget
//!    proportional to `w`.
//!
//! Final size clamped to `min_*` / `max_*`.
//!
//! Cross-axis sizing: `Fixed(n)` → `n`; `Percent(p)` → `container_cross * p / 100`;
//!  `Flex | Auto` → stretch to
//! container; clamped by min/max.
//!
//! IFC detection: if `id` has `display: inline` element children,
//! skip flex distribution entirely. Inline children get zero-sized
//! layout rects (paint reads `inline_layout` from the block's `ext`
//! instead).
//!
//! ## Module layout
//!
//! - `mod.rs` — [`layout_children`] (the IFC / text-leaf / block / flex
//!   dispatch) and [`layout_flex_children`], the orchestrator that
//!   threads one flex line through the pieces below in spec order.
//! - [`main_axis`] — per-item main-size gathering (`ChildMain`), the
//!   §9.7 grow / shrink freeze loops, and the lazy §4.5 auto-min floor.
//! - [`cross`] — §9.4 cross-size determination, `aspect-ratio`, and
//!   §9.5 auto cross margins / offset.
//! - [`placement`] — §9.5 main-axis placement: auto main margins,
//!   gaps, cursor advance, and the `layout_node` recursion per item.
//! - [`collapse`] — the flex-specific `border-collapse: collapse`
//!   rules: parent-edge inset and one-cell sibling overlap.
//!
//! [`intrinsic::intrinsic_size`]: super::intrinsic::intrinsic_size

mod collapse;
mod cross;
mod main_axis;
mod placement;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect};
use crate::render::inline::compute_inline_layout;
use crate::style::ComputedStyle;

use super::ifc::is_ifc_block;
use super::{element_children_of, layout_node};
use collapse::SiblingOverlap;
use main_axis::{MainAxisBudget, collect_main_axis_items, resolve_flexible_lengths};
use placement::{AutoMainMargins, FlexLine, place_items};

/// Lay out the **element** children of `id` inside `container`, using
/// `computed`'s `direction`, `gap`, and the children's own sizes.
///
/// Returns `Some(BlockMeasurement)` ONLY when this dispatch went
/// through the `Flow::Block` arm — that's the only path where
/// `layout_node` should override the element's `Auto` height with
/// the measured content extent. IFC / pure-text-leaf / flex paths
/// return `None` because their own height already lands correctly
/// (IFC + pure-text via `inline_layout.height()`; flex via
/// distribution from the parent's main-axis budget).
pub(super) fn layout_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    computed: &ComputedStyle,
) -> Option<super::block::BlockMeasurement> {
    // Drop anonymous-block boxes from a PRIOR layout up front. Only the
    // block-flow arm (`layout_block_children`) repopulates them; the IFC,
    // pure-text-leaf, and flex paths never produce anon boxes. Without this an
    // element that *transitions into* one of those paths — e.g. a block that
    // had an element child (so its inline run was wrapped in an anon box), then
    // becomes a pure-text leaf when that child is removed — keeps painting the
    // stale boxes at their old position (PAINT-RELATIVE-ABSPOS-DOUBLE: the
    // show/hide chip's glyph echoing at its previous slot after the dropdown
    // child was dropped). Clearing here, once, covers every dispatch arm.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.anonymous_blocks.clear();
    }

    // IFC block: inline element children don't participate in flex
    // layout — they're painted by the inline flow pass. Give each a
    // zero-sized layout rect (hit tests and debug tools shouldn't
    // crash on missing data; paint reads the parent's inline_layout
    // instead).
    if is_ifc_block(dom, id) {
        for child in element_children_of(dom, id) {
            if let Some(ext) = dom.node_mut(child).ext_mut() {
                ext.layout = LayoutRect::new(container.x, container.y, 0, 0);
                ext.content_layout = ext.layout;
                ext.layout_dirty = false;
                ext.margin_chain = None;
            }
        }
        // Compute + store the inline layout at the block's final
        // content width. Paint reads this back directly.
        let inline_layout = compute_inline_layout(dom, id, container.width);
        super::positioning::record_static_positions_in_ifc(
            dom,
            id,
            &inline_layout,
            crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(container),
        );
        // Atomic inline-block fragments (`<button>` in
        // `<p>hi <button>X</button> ok</p>`) need their layout rect
        // written so hit-test descends into them, and need
        // `layout_node` recursion so their own subtrees lay out
        // (text wrap, pseudos, descendants). Snapshot fragments
        // first to satisfy the borrow checker.
        let atoms = crate::render::inline::atomic_placements(&inline_layout, container);
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.inline_layout = Some(inline_layout);
        }
        for (atom_id, atom_rect) in atoms {
            layout_node(dom, atom_id, atom_rect, container.width);
        }
        // IFC height is the line count — block-flow auto-height
        // resolution uses this if the IFC block has `height: auto`.
        // IFC paths don't return a `BlockMeasurement` because
        // `layout_node`'s height override is gated on `Flow::Block`
        // anyway; passing `None` keeps the invariant in the type
        // system rather than relying on the caller's guard.
        return None;
    }

    // Pure-text leaf block (e.g. `<textarea>`, `<input>`, `<p>only
    // text</p>`). Any element with a direct text-node child and no
    // element children. It's not an IFC per `is_ifc_block`'s carve-
    // out (paint routing for `::before` / `::after` chrome), but its
    // rendered text still needs to wrap AND its caret needs an
    // inline-flow container to anchor to.
    //
    // Empty text (e.g. an unsubmitted `<input>` / `<textarea>`)
    // still qualifies: the caret has to land somewhere, so the
    // inline_layout is computed even when its lines list is empty
    // or a single empty line. Paint reads it back to position the
    // REVERSED caret cell.
    let has_text_child = dom
        .node(id)
        .child_nodes()
        .any(|c| c.node_type() == rdom_core::NodeType::Text);
    // Only *in-flow* element children disqualify the pure-text-leaf path:
    // out-of-flow children (`position: absolute|fixed`) don't participate in the
    // block/inline mix, so a "text + an absolutely-positioned child" element
    // (e.g. a chip with an absolute dropdown) is still a text leaf — it must use
    // its own `inline_layout` (so `::before`/`::after` + own text paint once via
    // Path 3, not duplicated by an anonymous block; see TREE-BFC-PSEUDO-1).
    let no_in_flow_element_children = element_children_of(dom, id)
        .iter()
        .all(|&c| !super::is_in_flow(dom, c));
    if has_text_child && no_in_flow_element_children {
        let inline_layout = compute_inline_layout(dom, id, container.width);
        super::positioning::record_static_positions_in_ifc(
            dom,
            id,
            &inline_layout,
            crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(container),
        );
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.inline_layout = Some(inline_layout);
        }
        return None;
    }

    if let Some(ext) = dom.node_mut(id).ext_mut() {
        // Clear stale inline layout — the element may have
        // transitioned back to block via cascade.
        ext.inline_layout = None;
    }

    // BFC-1 Phase 4.1 — dispatch on cascaded `flow`. Default flow
    // is Block (set by Phase 1's cascade machinery), so semantic
    // HTML (`<div><h1><p></p></div>`) routes to
    // `layout_block_children` for CSS 2.1 §10 normal-flow stacking.
    // Authors opt into flex with `display: flex` (which the parser
    // maps to Display::Block + Flow::Flex per CSS3 Display Module).
    //
    // Note: this branch runs ONLY after the IFC + pure-text-leaf
    // carve-outs above. Both of those paths must stay above the
    // dispatch — they're not parameterized by Flow.
    match computed.flow {
        crate::layout::Flow::Block => {
            // Stale anon boxes from a prior flex layout: clear so
            // the new block layout starts fresh. Anon boxes will
            // be repopulated by `layout_block_children`. Only this
            // arm produces a `BlockMeasurement` that
            // `layout_node`'s `Auto`-height override consumes.
            return Some(super::block::layout_block_children(
                dom, id, container, computed,
            ));
        }
        crate::layout::Flow::Flex => {
            // Fall through to flex distribution. (Anon boxes already
            // cleared at the top of `layout_children`.)
        }
    }

    // Filter out children that don't participate in normal flow:
    //
    // - `display: none` — invisible to layout and paint.
    // - `position: absolute` / `position: fixed` (M2) — removed
    //   from flow so the parent's flex distribution doesn't see
    //   them. Their final layout rect is filled in by phase-2
    //   placement after this pass returns.
    //
    // Their `LayoutRect` stays at the default zero from
    // `TuiExt::default` until something writes to it.
    let children: Vec<NodeId> = element_children_of(dom, id)
        .into_iter()
        .filter(|&c| super::is_in_flow(dom, c))
        .collect();
    // `D-M2-2`: a positioned child's static position in a flex
    // container is the content box's start — Flexbox §4.1 places it as
    // the sole item; `justify-content` / `align-items` are not applied
    // (DIVERGENCES). Both scroll offsets apply, as they do to the
    // in-flow items.
    let (static_x, static_y) = dom.node(id).ext().map_or((container.x, container.y), |e| {
        (
            container.x - e.scroll_x as i32,
            container.y - e.scroll_y as i32,
        )
    });
    for n in super::positioning::out_of_flow_positioned_children(dom, id) {
        super::positioning::record_static_position(dom, n, static_x, static_y);
    }
    layout_flex_children(dom, &children, container, computed);
    // Flex distribution sets each child's outer rect inside the
    // container; the container's own height was determined by its
    // parent's distribution / its declared size. Auto height on a
    // flex container resolves via the parent's distribution +
    // `intrinsic_size`, not via a children-walk. Return `None` so
    // `layout_node` leaves our height alone.
    None
}

/// Lay out one flex line: `children` (already filtered to in-flow
/// items) inside `container`, driven by `parent`'s `direction`, `gap`
/// and `border-collapse`.
///
/// Runs the CSS Flexible Box algorithm in spec order — gather main
/// sizes (§9.2), determine free space and auto-margin claims (§9.7 /
/// §9.5), resolve flexible lengths (§9.7), then place each item along
/// the main axis and size it on the cross axis (§9.4–§9.6) — with the
/// per-concern math in the sibling modules.
pub(super) fn layout_flex_children(
    dom: &mut Dom<TuiExt>,
    children: &[NodeId],
    container: LayoutRect,
    parent: &ComputedStyle,
) {
    if children.is_empty() {
        return;
    }

    let direction = parent.direction;
    let gap = super::resolve_gap(parent, container, direction);

    let container = collapse::inset_container_for_children(dom, children, parent, container);

    // Main-axis budget for distribution (cells available to all
    // children + gaps).
    let main_budget: u16 = match direction {
        Direction::Row => container.width,
        Direction::Column => container.height,
    };
    let cross_budget: u16 = match direction {
        Direction::Row => container.height,
        Direction::Column => container.width,
    };

    let line = collect_main_axis_items(dom, children, direction, main_budget, cross_budget);

    // Gap total = (n - 1) * gap.
    let gap_total = gap.saturating_mul((children.len() as u16).saturating_sub(1));

    let overlap = SiblingOverlap::new(parent, gap, direction);
    let overlap_savings = overlap.savings(dom, children);

    // Space left after sizes + gaps + non-auto margins, plus the
    // cells reclaimed by sibling-overlap.
    let remaining = (i32::from(main_budget) - line.consumed_fixed - i32::from(gap_total)
        + i32::from(overlap_savings))
    .clamp(0, i32::from(u16::MAX)) as u16;

    // CSS rule for flex auto-margins: when free space > 0 AND any
    // auto margins exist on the main axis, those margins consume the
    // free space; flex-grow does NOT grow. When free space ≤ 0, autos
    // resolve to 0 and flex-shrink takes over. (M5.3b)
    let auto_main_count = line.auto_main_count;
    let auto_margins = AutoMainMargins {
        share: (remaining as u32).checked_div(auto_main_count).unwrap_or(0) as u16,
        remainder: (remaining as u32).checked_rem(auto_main_count).unwrap_or(0),
    };
    let flex_remaining: u16 = if auto_main_count > 0 { 0 } else { remaining };

    let net_budget = (main_budget as i32) - (gap_total as i32) + (overlap_savings as i32);
    let final_main = resolve_flexible_lengths(
        dom,
        &line.items,
        direction,
        MainAxisBudget {
            main: main_budget,
            cross: cross_budget,
            flex_remaining,
            net: net_budget,
        },
    );

    place_items(
        dom,
        children,
        FlexLine {
            items: &line.items,
            final_main: &final_main,
            container,
            direction,
            gap,
            cross_budget,
            auto_margins,
            overlap,
        },
    );
}
