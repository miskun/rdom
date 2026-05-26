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

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout;
use crate::style::ComputedStyle;

use super::ifc::is_ifc_block;
use super::intrinsic::{content_min_size, intrinsic_size};
use super::{element_children_of, layout_node, parent_scroll};

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
            }
        }
        // Compute + store the inline layout at the block's final
        // content width. Paint reads this back directly.
        let inline_layout = compute_inline_layout(dom, id, container.width);
        // Atomic inline-block fragments (`<button>` in
        // `<p>hi <button>X</button> ok</p>`) need their layout rect
        // written so hit-test descends into them, and need
        // `layout_node` recursion so their own subtrees lay out
        // (text wrap, pseudos, descendants). Snapshot fragments
        // first to satisfy the borrow checker.
        let mut atoms: Vec<(NodeId, LayoutRect)> = Vec::new();
        for (line_idx, line) in inline_layout.lines.iter().enumerate() {
            let line_y = container.y + line_idx as i32;
            for fragment in &line.fragments {
                if !fragment.atomic {
                    continue;
                }
                atoms.push((
                    fragment.node,
                    LayoutRect::new(container.x + fragment.x as i32, line_y, fragment.width, 1),
                ));
            }
        }
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.inline_layout = Some(inline_layout);
        }
        for (atom_id, atom_rect) in atoms {
            layout_node(dom, atom_id, atom_rect);
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
    if has_text_child && element_children_of(dom, id).is_empty() {
        let inline_layout = compute_inline_layout(dom, id, container.width);
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
            // Fall through to flex distribution. Stale anon boxes
            // from a prior block layout get cleared here.
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.anonymous_blocks.clear();
            }
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
        .filter(|&c| {
            let computed = dom.node(c).ext().and_then(|e| e.computed.as_ref());
            match computed {
                Some(s) => {
                    s.display != crate::layout::Display::None
                        && !matches!(
                            s.position,
                            crate::layout::Position::Absolute | crate::layout::Position::Fixed
                        )
                }
                None => true,
            }
        })
        .collect();
    layout_flex_children(dom, &children, container, computed);
    // Flex distribution sets each child's outer rect inside the
    // container; the container's own height was determined by its
    // parent's distribution / its declared size. Auto height on a
    // flex container resolves via the parent's distribution +
    // `intrinsic_size`, not via a children-walk. Return `None` so
    // `layout_node` leaves our height alone.
    None
}

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
    let gap = parent.gap;

    // ── Parent-child border-collapse inset ─────────────────────────
    //
    // Under `border-collapse: collapse`, `compute_content_area_collapsed`
    // flattens the parent's content area to its outer rect — children's
    // outer rects then extend into the parent's border ring (so a
    // bordered child's first cell coincides with the parent's first
    // border cell, sharing one paint surface — the table-cell model).
    //
    // That sharing is only correct when the first/last child ACTUALLY
    // HAS A BORDER to share. If the first child is content-bearing
    // (no own border), its content would land on the parent's painted
    // border row and disappear under the border glyph. Surfaced
    // visually by the showcase chrome: `<header>` inside an `<app>`
    // with collapse + own border had its `<h1>` text painted at the
    // shared border row.
    //
    // Per-edge fix: if the first child along the main axis has no
    // border, push that edge's start back by 1 so the first child's
    // content area sits below the parent's border row. Same for the
    // last child along the main axis. Cross-axis insets follow the
    // same logic. Pre-scan one element child each direction; correct
    // for the common case (table cells vs. content-bearing chrome
    // panels) without touching `compute_content_area_collapsed`.
    let (top_inset, bot_inset, left_inset, right_inset) =
        collapse_parent_edge_insets(dom, children, parent);
    let container = LayoutRect::new(
        container.x + left_inset as i32,
        container.y + top_inset as i32,
        container.width.saturating_sub(left_inset + right_inset),
        container.height.saturating_sub(top_inset + bot_inset),
    );

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

    // Gather per-child (Size, min, max, is_flex) tuples for the main
    // axis.
    let mut child_info: Vec<ChildMain> = Vec::with_capacity(children.len());
    let mut consumed_fixed: u16 = 0;
    let mut total_flex_weight: u32 = 0;
    let mut auto_main_count: u32 = 0;

    for &child in children {
        let c = dom
            .node(child)
            .computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial);
        let (main_size, min_raw, max) = match direction {
            Direction::Row => (c.width, c.min_width, c.max_width),
            Direction::Column => (c.height, c.min_height, c.max_height),
        };

        // Main-axis margins (M5.3b). Cells contribute to consumed
        // space; Auto absorbs remaining free space after flex
        // distribution (CSS rule).
        use crate::layout::MarginValue;
        let (main_start_m, main_end_m) = match direction {
            Direction::Row => (c.margin.left, c.margin.right),
            Direction::Column => (c.margin.top, c.margin.bottom),
        };
        let margin_consumed = match (main_start_m, main_end_m) {
            (MarginValue::Cells(a), MarginValue::Cells(b)) => {
                (a.max(0) as u16).saturating_add(b.max(0) as u16)
            }
            (MarginValue::Cells(a), MarginValue::Auto) => a.max(0) as u16,
            (MarginValue::Auto, MarginValue::Cells(b)) => b.max(0) as u16,
            (MarginValue::Auto, MarginValue::Auto) => 0,
        };
        consumed_fixed = consumed_fixed.saturating_add(margin_consumed);
        if matches!(main_start_m, MarginValue::Auto) {
            auto_main_count += 1;
        }
        if matches!(main_end_m, MarginValue::Auto) {
            auto_main_count += 1;
        }

        let natural = match &main_size {
            Size::Fixed(n) => MainNatural::Fixed(*n),
            Size::Flex(w) => {
                total_flex_weight += *w as u32;
                MainNatural::Flex(*w)
            }
            Size::Percent(p) => {
                // Percent resolves against the parent's main-axis
                // content area at layout time. Treated as a fixed
                // cell value once resolved — does NOT participate
                // in flex weight distribution.
                let resolved = ((main_budget as u32 * *p as u32) / 100).min(u16::MAX as u32) as u16;
                MainNatural::Fixed(resolved)
            }
            Size::Calc(expr) => {
                // Calc resolves against the same axis basis as
                // Percent — parent's main-axis content dimension.
                let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(main_budget as i32));
                let resolved = v.max(0).min(u16::MAX as i32) as u16;
                MainNatural::Fixed(resolved)
            }
            Size::Auto => {
                let intrinsic = intrinsic_size(dom, child, direction, cross_budget);
                MainNatural::Auto(intrinsic)
            }
        };

        // Resolve `min-width: auto` / `min-height: auto` → intrinsic
        // content size, per CSS Flexbox §4.5. Flex items default to
        // `min-*: auto` even when the author writes nothing —
        // that's the CSS contract. Without this floor, a flex
        // container that overflows would silently shrink its items
        // to zero cells (the M5-MIN-CONTENT-1 substrate bug).
        //
        // **Lazy resolution.** The auto-min only matters during
        // shrink (`total > net_budget`). For the first `clamp_size`
        // pass below, auto-min is mathematically ≤ natural for
        // every Size variant (Flex items have specified_cap = 0;
        // Fixed/Percent/Calc items have natural = specified_cap;
        // Auto items have natural = intrinsic ≥ content-min). So
        // we skip the content walk here — `min` carries only the
        // explicit `Cells(n)` floor for the first pass; the shrink
        // branch resolves Auto on demand for items it actually
        // shrinks.
        //
        // Authors that want strict zero shrink set `min-*: 0`
        // explicitly. Authors that want content-protection on a
        // grow item write the basis explicitly (`flex: 0 1 auto`
        // / `width: auto`) so the specified suggestion is
        // unbounded.
        //
        // Profile evidence: eager resolution added +47% to the
        // full-frame benchmark
        // (`benches/runtime.rs::bench_full_frame`); shrink-only
        // resolution recovers the cost for the non-overflowing
        // case (the common case).
        //
        // v1 approximates CSS min-content with intrinsic natural
        // size; strict min-content (longest-word width with wrap)
        // is a future polish tracked as `M5-MIN-CONTENT-2`.
        let min = match min_raw {
            None => None,
            Some(crate::layout::MinSize::Cells(n)) => Some(n),
            Some(crate::layout::MinSize::Auto) => None,
        };

        if let MainNatural::Fixed(n) | MainNatural::Auto(n) = natural {
            consumed_fixed = consumed_fixed.saturating_add(n);
        }

        child_info.push(ChildMain {
            id: child,
            main: natural,
            min,
            max,
            main_start_margin: main_start_m,
            main_end_margin: main_end_m,
        });
    }

    // Gap total = (n - 1) * gap.
    let gap_total = gap.saturating_mul((children.len() as u16).saturating_sub(1));

    // Under `border-collapse: collapse`, each pair of adjacent
    // bordered siblings shares one cell on their meeting edge. The
    // cursor advance subtracts 1 per overlap (see the placement
    // loop below), but flex sizing needs to know up-front so the
    // grow distribution uses ALL the available cells — otherwise
    // the saved cells appear as empty space at the parent's right
    // / bottom edge (the headline `border-collapse` bug from M5
    // gate review).
    // Per-edge effective border check (transparent intermediate
    // propagation) — adjacent siblings overlap when both have a
    // border on the shared edge, either directly or via
    // borderless container children that propagate the sharing
    // to bordered descendants.
    let (edge_i, edge_next) = match direction {
        Direction::Column => (CollapseEdge::Bottom, CollapseEdge::Top),
        Direction::Row => (CollapseEdge::Right, CollapseEdge::Left),
    };
    let mut overlap_savings: u16 = 0;
    if parent.border_collapse == crate::layout::BorderCollapse::Collapse {
        for i in 0..child_info.len().saturating_sub(1) {
            if has_effective_border_on_edge(dom, child_info[i].id, edge_i)
                && has_effective_border_on_edge(dom, child_info[i + 1].id, edge_next)
            {
                overlap_savings = overlap_savings.saturating_add(1);
            }
        }
    }

    // Space left after sizes + gaps + non-auto margins, plus the
    // cells reclaimed by sibling-overlap.
    let remaining = main_budget
        .saturating_sub(consumed_fixed)
        .saturating_sub(gap_total)
        .saturating_add(overlap_savings);

    // CSS rule for flex auto-margins: when free space > 0 AND any
    // auto margins exist on the main axis, those margins consume the
    // free space; flex-grow does NOT grow. When free space ≤ 0, autos
    // resolve to 0 and flex-shrink takes over. (M5.3b)
    let auto_share: u16 = (remaining as u32).checked_div(auto_main_count).unwrap_or(0) as u16;
    let auto_remainder: u32 = (remaining as u32).checked_rem(auto_main_count).unwrap_or(0);
    let flex_remaining: u16 = if auto_main_count > 0 { 0 } else { remaining };

    // Resolve each child's main-axis final size with min/max.
    //
    // Flex distribution uses a rolling (Bresenham-style) allocation
    // so the integer-division remainder doesn't get dropped. For
    // each flex child, the target cumulative flex size is computed
    // first, then the child's share is `target - already_allocated`.
    // This guarantees the sum of flex sizes equals `flex_remaining`
    // exactly when no min/max clamps fire. Without this, e.g. two
    // `Flex(1)` children with `flex_remaining = 31` would each get
    // `31 / 2 = 15`, summing to 30 and leaving 1 cell unallocated
    // as visible empty space — the bug surfaced by the
    // `border_collapse_demo` at odd terminal sizes.
    let mut final_main: Vec<u16> = {
        let mut accumulated_weight: u32 = 0;
        let mut accumulated_flex_size: u32 = 0;
        child_info
            .iter()
            .map(|ci| {
                let natural = match ci.main {
                    MainNatural::Fixed(n) | MainNatural::Auto(n) => n,
                    MainNatural::Flex(w) => {
                        accumulated_weight = accumulated_weight.saturating_add(w as u32);
                        let target = (flex_remaining as u32)
                            .saturating_mul(accumulated_weight)
                            .checked_div(total_flex_weight)
                            .unwrap_or(0);
                        let share = target.saturating_sub(accumulated_flex_size) as u16;
                        accumulated_flex_size = target;
                        share
                    }
                };
                clamp_size(natural, ci.min, ci.max)
            })
            .collect()
    };

    // ── flex-shrink ──────────────────────────────────────────────
    //
    // When the sum of children's declared sizes (+ gaps − overlap)
    // exceeds the main-axis budget, CSS distributes the overflow
    // proportional to `flex_shrink * basis` across shrinkable
    // children. Default `flex_shrink: 1` makes overflow gracefully
    // shrink-to-fit instead of clipping past the parent's edge —
    // the behavior every CSS author expects from
    // `height: 100% on a flex child` (the showcase chrome case).
    //
    // Bresenham-style accumulation keeps the per-child integer
    // shrink amounts exact (sum-of-shares matches the overflow
    // total, no off-by-one at the last child).
    let net_budget = (main_budget as i32) - (gap_total as i32) + (overlap_savings as i32);
    let total: i32 = final_main.iter().map(|&n| n as i32).sum();
    if total > net_budget && net_budget > 0 {
        let overflow = (total - net_budget) as u32;
        let shrink_of = |ci: &ChildMain| -> u32 {
            dom.node(ci.id)
                .computed()
                .map(|c| c.flex_shrink as u32)
                .unwrap_or(1)
        };
        let total_shrink_basis: u32 = child_info
            .iter()
            .zip(final_main.iter())
            .map(|(ci, &size)| (size as u32) * shrink_of(ci))
            .sum();
        if let Some(divisor) = std::num::NonZeroU32::new(total_shrink_basis) {
            let mut accumulated_basis: u32 = 0;
            let mut accumulated_shrink: u32 = 0;
            for (i, ci) in child_info.iter().enumerate() {
                let shrink = shrink_of(ci);
                if shrink == 0 {
                    continue;
                }
                accumulated_basis += (final_main[i] as u32) * shrink;
                let target_total_shrink = (accumulated_basis * overflow) / divisor.get();
                let my_shrink = target_total_shrink.saturating_sub(accumulated_shrink) as u16;
                accumulated_shrink = target_total_shrink;
                let new_size = final_main[i].saturating_sub(my_shrink);
                // Honor min clamp — child can't shrink below its
                // `min-width` / `min-height`. Explicit `Cells(n)` is
                // stored in `ci.min`; the auto-min (implicit or
                // explicit `Auto`) is resolved lazily here per CSS
                // Flexbox §4.5.
                let floor = ci.min.unwrap_or_else(|| {
                    resolve_auto_min(dom, ci.id, direction, main_budget, cross_budget)
                });
                final_main[i] = new_size.max(floor);
            }
        }
    }

    // Position each child along main axis, scrolling by parent's
    // scroll offset.
    let scroll_main = parent_scroll(dom, children, direction);

    let mut main_cursor: i32 = match direction {
        Direction::Row => container.x - scroll_main,
        Direction::Column => container.y - scroll_main,
    };

    let child_list: Vec<(NodeId, u16)> = child_info
        .iter()
        .map(|ci| ci.id)
        .zip(final_main.iter().copied())
        .collect();

    // Distribute the remainder (from integer division of auto_share)
    // to the first few auto margins so the totals add back up exactly.
    let mut autos_consumed: u32 = 0;
    let resolve_auto = |consumed: &mut u32| -> u16 {
        let extra = if *consumed < auto_remainder { 1 } else { 0 };
        *consumed += 1;
        auto_share.saturating_add(extra)
    };

    for (i, (child_id, size)) in child_list.iter().enumerate() {
        let child_computed = dom
            .node(*child_id)
            .computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial);

        // Resolve this child's main-axis start and end margins.
        use crate::layout::MarginValue;
        let main_start_cells = match child_info[i].main_start_margin {
            MarginValue::Cells(n) => n.max(0) as u16,
            MarginValue::Auto => resolve_auto(&mut autos_consumed),
        };
        let main_end_cells = match child_info[i].main_end_margin {
            MarginValue::Cells(n) => n.max(0) as u16,
            MarginValue::Auto => resolve_auto(&mut autos_consumed),
        };
        main_cursor = main_cursor.saturating_add(main_start_cells as i32);

        // Whether the child's main-axis size was declared `Auto` —
        // needed so `resolve_cross_size` knows whether to apply
        // aspect-ratio (which requires the main axis to be explicit).
        let main_was_auto = matches!(child_info[i].main, MainNatural::Auto(_));
        let cross_size = resolve_cross_size(
            dom,
            *child_id,
            &child_computed,
            cross_budget,
            direction,
            *size,
            main_was_auto,
        );

        let child_rect = match direction {
            Direction::Row => LayoutRect::new(main_cursor, container.y, *size, cross_size),
            Direction::Column => LayoutRect::new(container.x, main_cursor, cross_size, *size),
        };

        layout_node(dom, *child_id, child_rect);

        // Advance cursor past this child + main-end margin + gap.
        main_cursor = main_cursor.saturating_add(*size as i32);
        main_cursor = main_cursor.saturating_add(main_end_cells as i32);
        if i + 1 < child_list.len() {
            main_cursor = main_cursor.saturating_add(gap as i32);
            // M5.5b + Finding 2 — sibling border overlap under
            // `border-collapse: collapse`. When the parent has
            // collapse active AND both this child and the next
            // have a border on the shared edge (directly or via
            // transparent intermediate containers), they share one
            // cell at the junction: pull the cursor back by 1.
            if parent.border_collapse == crate::layout::BorderCollapse::Collapse
                && has_effective_border_on_edge(dom, child_info[i].id, edge_i)
                && has_effective_border_on_edge(dom, child_info[i + 1].id, edge_next)
            {
                main_cursor = main_cursor.saturating_sub(1);
            }
        }
    }
}

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

/// Does `id` have a border on `edge` for `border-collapse`
/// sharing purposes — directly, OR via transparent borderless
/// container intermediates that propagate the sharing through
/// to bordered descendants?
///
/// Closes the gap surfaced by the M2 visual review: a layout
/// like `<app border collapse> > <header border> + <body no-border> > <sidebar border> + <main border>`
/// should share `<header>`'s bottom with `<sidebar>` / `<main>`'s
/// tops through the transparent `<body>`. Same shape as the
/// `collapse_parent_edge_insets` content-inset logic, applied to
/// the sibling-overlap axis.
///
/// Recursion direction:
///
/// - Column-direction container: only first column-child touches
///   parent's top edge; only last touches parent's bottom edge;
///   any child can touch left or right (children span the cross
///   axis).
/// - Row-direction container: mirror of the column case.
pub(super) fn has_effective_border_on_edge(
    dom: &Dom<TuiExt>,
    id: NodeId,
    edge: CollapseEdge,
) -> bool {
    use crate::layout::Border;
    let computed = dom
        .node(id)
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);

    let own_has_edge = match edge {
        CollapseEdge::Top => computed.border.top,
        CollapseEdge::Bottom => computed.border.bottom,
        CollapseEdge::Left => computed.border.left,
        CollapseEdge::Right => computed.border.right,
    };
    if own_has_edge {
        return true;
    }
    if computed.border != Border::none() {
        // Has some border but not the queried edge — opaque on
        // this edge. Don't look through.
        return false;
    }

    // Borderless — transparent. Look through to children.
    let children: Vec<NodeId> = super::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| {
            let cc = dom.node(c).computed();
            cc.is_none_or(|s| {
                s.display != crate::layout::Display::None
                    && !matches!(
                        s.position,
                        crate::layout::Position::Absolute | crate::layout::Position::Fixed
                    )
            })
        })
        .collect();
    if children.is_empty() {
        return false;
    }

    let dir = computed.direction;
    match (dir, edge) {
        (Direction::Column, CollapseEdge::Top) => {
            has_effective_border_on_edge(dom, children[0], CollapseEdge::Top)
        }
        (Direction::Column, CollapseEdge::Bottom) => {
            has_effective_border_on_edge(dom, *children.last().unwrap(), CollapseEdge::Bottom)
        }
        (Direction::Column, CollapseEdge::Left | CollapseEdge::Right) => children
            .iter()
            .any(|&c| has_effective_border_on_edge(dom, c, edge)),
        (Direction::Row, CollapseEdge::Top | CollapseEdge::Bottom) => children
            .iter()
            .any(|&c| has_effective_border_on_edge(dom, c, edge)),
        (Direction::Row, CollapseEdge::Left) => {
            has_effective_border_on_edge(dom, children[0], CollapseEdge::Left)
        }
        (Direction::Row, CollapseEdge::Right) => {
            has_effective_border_on_edge(dom, *children.last().unwrap(), CollapseEdge::Right)
        }
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
    let parent_has_top = parent.border.top;
    let parent_has_bottom = parent.border.bottom;
    let parent_has_left = parent.border.left;
    let parent_has_right = parent.border.right;
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

/// Compute the cross-axis cell count from the main-axis cell count and
/// an `aspect-ratio: w/h` value. `Row` direction: cross is height, so
/// `height = width * h / w`. `Column` direction: cross is width, so
/// `width = height * w / h`. Half-to-even rounding to integer cells.
fn aspect_cross_from_main(
    main: u16,
    ratio: crate::layout::AspectRatio,
    direction: Direction,
) -> u16 {
    let r = ratio.as_f32();
    let cross_f = match direction {
        Direction::Row => (main as f32) / r,
        Direction::Column => (main as f32) * r,
    };
    if cross_f.is_finite() {
        cross_f.max(0.0).round_ties_even() as u16
    } else {
        0
    }
}

struct ChildMain {
    id: NodeId,
    main: MainNatural,
    /// Explicit `min: <n>` cell floor, or `None` for "use auto-min
    /// resolved lazily during shrink". `None` covers both an unset
    /// min (CSS spec default for flex items) and an explicit
    /// `min: auto` — they share the same auto resolution path per
    /// CSS Flexbox §4.5.
    min: Option<u16>,
    max: Option<u16>,
    main_start_margin: crate::layout::MarginValue,
    main_end_margin: crate::layout::MarginValue,
}

/// Compute the auto-min floor for `id` along `direction`, per CSS
/// Flexbox §4.5. Called from the shrink branch when an item with
/// implicit/auto min is about to be shrunk — eager resolution
/// during the natural-size pass would walk every flex item's
/// subtree every layout (the +47% regression observed in the
/// full-frame benchmark), so we defer until we know the item is
/// actually shrinking.
fn resolve_auto_min(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    main_budget: u16,
    cross_budget: u16,
) -> u16 {
    let computed = match dom.node(id).computed() {
        Some(c) => c.clone(),
        None => return 0,
    };
    let main_size = match direction {
        Direction::Row => &computed.width,
        Direction::Column => &computed.height,
    };
    let overflow_on_axis = match direction {
        Direction::Row => computed.overflow_x,
        Direction::Column => computed.overflow_y,
    };
    // CSS §4.5 exception: non-visible overflow drops the floor to 0
    // — items inside a scroll container are allowed to be sized
    // below their content.
    if overflow_on_axis != crate::layout::Overflow::Visible {
        return 0;
    }
    // Specified size suggestion per spec.
    let specified_cap: Option<u16> = match main_size {
        Size::Fixed(n) => Some(*n),
        Size::Percent(p) => {
            Some(((main_budget as u32 * *p as u32) / 100).min(u16::MAX as u32) as u16)
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(main_budget as i32));
            Some(v.max(0).min(u16::MAX as i32) as u16)
        }
        Size::Flex(_) => Some(0),
        Size::Auto => None,
    };
    // `flex: N` (basis 0%) trivially has specified=0, so auto-min
    // = min(content, 0) = 0. Skip the content walk.
    if matches!(specified_cap, Some(0)) {
        return 0;
    }
    let content = content_min_size(dom, id, direction, cross_budget);
    match specified_cap {
        Some(cap) => content.min(cap),
        None => content,
    }
}

enum MainNatural {
    Fixed(u16),
    Flex(u16),
    Auto(u16),
}

/// Compute the cross-axis size for a child given the container cross
/// budget, the parent's flex direction, and the child's resolved main
/// size. Rules:
///
/// - `Fixed(n)` → `n` (explicit wins).
/// - `Flex(_)` → stretch to fill the cross budget (explicit grow).
/// - `Auto` →
///   - If `aspect-ratio` is set AND the child's main axis was *not*
///     `Auto`, compute cross from main via the ratio (CSS Sizing 4
///     §3.2). Half-to-even rounding to integer cells.
///   - Else if `display: inline-block` → intrinsic content size on the
///     cross axis.
///   - Else → stretch to fill the cross budget.
///
/// Then clamps by `min` / `max`.
fn resolve_cross_size(
    dom: &Dom<TuiExt>,
    child_id: NodeId,
    computed: &ComputedStyle,
    container_cross: u16,
    direction: Direction,
    main_size: u16,
    main_was_auto: bool,
) -> u16 {
    let (cross_size, min_raw, max) = match direction {
        Direction::Row => (&computed.height, computed.min_height, computed.max_height),
        Direction::Column => (&computed.width, computed.min_width, computed.max_width),
    };
    let cross_dir = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    let natural = match cross_size {
        Size::Fixed(n) => *n,
        Size::Flex(_) => container_cross,
        Size::Percent(p) => {
            // Cross-axis percent resolves against the container's
            // cross-axis dimension.
            ((container_cross as u32 * *p as u32) / 100).min(u16::MAX as u32) as u16
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(container_cross as i32));
            v.max(0).min(u16::MAX as i32) as u16
        }
        Size::Auto => {
            if let Some(ratio) = computed.aspect_ratio
                && !main_was_auto
                && main_size > 0
            {
                aspect_cross_from_main(main_size, ratio, direction)
            } else if computed.display == crate::layout::Display::InlineBlock {
                // Cross-axis intrinsic measurement. `intrinsic_size`'s
                // `direction` argument means "measure along this axis";
                // we want the axis perpendicular to the parent's flex
                // direction. The `cross_budget` argument passed to
                // `intrinsic_size` is for IFC wrap; for the inline-
                // block's own cross-axis sizing we pass the container
                // cross size — a conservative budget that's correct
                // for non-IFC inline-blocks (the common case).
                intrinsic_size(dom, child_id, cross_dir, container_cross)
            } else {
                container_cross
            }
        }
    };
    // Cross axis intentionally does NOT carry an auto-min content
    // floor. CSS Flexbox §4.5's content-based min applies to the
    // MAIN axis only, where a flex distribution can drive an item
    // below its content size. The cross axis in rdom has no shrink
    // distribution — items either stretch to fill the container
    // (`Size::Auto` / `Flex`), take a declared value (`Fixed` /
    // `Percent` / `Calc`), or honor an explicit `min-*: auto` opt-
    // in. There is no path that would silently collapse cross-axis
    // sizes, so no floor is needed. Adding one would force items
    // to GROW past their natural cross size — which would break
    // IFC wrap (a narrow column container's wider-content child
    // would balloon to its content width, defeating the wrap).
    let min = match min_raw {
        None => None,
        Some(crate::layout::MinSize::Cells(n)) => Some(n),
        Some(crate::layout::MinSize::Auto) => {
            Some(intrinsic_size(dom, child_id, cross_dir, container_cross))
        }
    };
    clamp_size(natural, min, max)
}
