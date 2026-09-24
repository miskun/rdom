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

use super::border_collapse::{
    CollapseEdge, collapse_parent_edge_insets, has_effective_border_on_edge,
};
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
            }
        }
        // Compute + store the inline layout at the block's final
        // content width. Paint reads this back directly.
        let inline_layout = compute_inline_layout(dom, id, container.width);
        super::positioning::record_static_positions_in_ifc(dom, id, &inline_layout, container);
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
        super::positioning::record_static_positions_in_ifc(dom, id, &inline_layout, container);
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
    // (DIVERGENCES). Scroll follows the main axis, as
    // `layout_flex_children` does for the in-flow items.
    let scroll_main = dom.node(id).ext().map_or(0, |e| match computed.direction {
        Direction::Row => e.scroll_x as i32,
        Direction::Column => e.scroll_y as i32,
    });
    let (static_x, static_y) = match computed.direction {
        Direction::Row => (container.x - scroll_main, container.y),
        Direction::Column => (container.x, container.y - scroll_main),
    };
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
    // Signed: a negative margin frees main-axis space (Flexbox §9.7
    // counts outer sizes; CSS margins may be negative).
    let mut consumed_fixed: i32 = 0;
    let mut auto_main_count: u32 = 0;

    for &child in children {
        let c = dom
            .node(child)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let (main_size, min_raw, max) = match direction {
            Direction::Row => (c.width.clone(), c.min_width, c.max_width),
            Direction::Column => (c.height.clone(), c.min_height, c.max_height),
        };
        // TABLE-COLSYNC-1: a table cell's *used* column width — computed by
        // `size_columns` from the column's author widths + content and stored
        // on the cell's ext (layout output, NOT author `inline_style`) —
        // overrides the normal main-size resolution so every cell in the
        // column lines up. A width drives the Row main axis only.
        let main_size = match (
            direction,
            dom.node(child).ext().and_then(|e| e.table_used_width),
        ) {
            (Direction::Row, Some(w)) => Size::Fixed(w),
            _ => main_size,
        };

        // Main-axis margins (M5.3b). Cells contribute to consumed
        // space; Auto absorbs remaining free space after flex
        // distribution (CSS rule).
        use crate::layout::MarginValue;
        let (main_start_m, main_end_m) = match direction {
            Direction::Row => (c.margin.left.clone(), c.margin.right.clone()),
            Direction::Column => (c.margin.top.clone(), c.margin.bottom.clone()),
        };
        // Resolve margin values (including Calc-with-percent) against
        // the parent's main-axis budget — CSS 2.1 §8.3 always uses
        // width, so `main_budget` is correct for Row main, and we
        // use `cross_budget` for Column main (= parent's width).
        let main_cb_w = match direction {
            Direction::Row => main_budget,
            Direction::Column => cross_budget,
        };
        let margin_consumed = |m: &MarginValue| -> i32 {
            if m.is_auto() {
                0
            } else {
                i32::from(m.resolve(main_cb_w))
            }
        };
        consumed_fixed += margin_consumed(&main_start_m) + margin_consumed(&main_end_m);
        if matches!(main_start_m, MarginValue::Auto) {
            auto_main_count += 1;
        }
        if matches!(main_end_m, MarginValue::Auto) {
            auto_main_count += 1;
        }

        let natural = match &main_size {
            Size::Fixed(n) => MainNatural::Fixed(*n),
            Size::Flex(w) => MainNatural::Flex(*w),
            Size::Percent(p) => {
                // Percent resolves against the parent's main-axis
                // content area at layout time. Treated as a fixed
                // cell value once resolved — does NOT participate
                // in flex weight distribution.
                let resolved =
                    Size::percent_of(main_budget as i32, *p).clamp(0, u16::MAX as i32) as u16;
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
                // The container's inner width is definite here, so the
                // item's percent padding / margins resolve against it.
                let intrinsic = intrinsic_size(dom, child, direction, cross_budget, main_cb_w);
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
            consumed_fixed += i32::from(n);
        }

        // Pre-resolve `Calc` margins to `Cells` here so the placement
        // loop below can match on `Cells | Auto` exhaustively. Calc
        // percent resolves against the parent's main-axis width
        // (CSS 2.1 §8.3) — `main_cb_w` computed above.
        let resolve_margin = |m: MarginValue| -> MarginValue {
            match m {
                MarginValue::Auto => MarginValue::Auto,
                MarginValue::Cells(n) => MarginValue::Cells(n),
                MarginValue::Calc(_) => MarginValue::Cells(m.resolve(main_cb_w)),
            }
        };
        child_info.push(ChildMain {
            id: child,
            main: natural,
            min,
            max,
            main_start_margin: resolve_margin(main_start_m),
            main_end_margin: resolve_margin(main_end_m),
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
    // / bottom edge.
    //
    // **BORDER-MODEL-1 gap-honoring rule.** When the author writes
    // `gap > 0`, the gap is sacred: it produces a visible cell
    // between siblings and there is nothing adjacent to merge. So
    // `overlap_savings` only fires when `gap == 0` AND the parent
    // declares `collapse`. With `gap > 0`, the gap and collapse
    // coexist orthogonally — `collapse` becomes a no-op for that
    // sibling pair. Documented as the 2×2 outcome grid in
    // `DIVERGENCES.md`.
    let (edge_i, edge_next) = match direction {
        Direction::Column => (CollapseEdge::Bottom, CollapseEdge::Top),
        Direction::Row => (CollapseEdge::Right, CollapseEdge::Left),
    };
    let mut overlap_savings: u16 = 0;
    if parent.border_collapse == crate::layout::BorderCollapse::Collapse && gap == 0 {
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
    let remaining = (i32::from(main_budget) - consumed_fixed - i32::from(gap_total)
        + i32::from(overlap_savings))
    .clamp(0, i32::from(u16::MAX)) as u16;

    // CSS rule for flex auto-margins: when free space > 0 AND any
    // auto margins exist on the main axis, those margins consume the
    // free space; flex-grow does NOT grow. When free space ≤ 0, autos
    // resolve to 0 and flex-shrink takes over. (M5.3b)
    let auto_share: u16 = (remaining as u32).checked_div(auto_main_count).unwrap_or(0) as u16;
    let auto_remainder: u32 = (remaining as u32).checked_rem(auto_main_count).unwrap_or(0);
    let flex_remaining: u16 = if auto_main_count > 0 { 0 } else { remaining };

    // Resolve each child's main-axis final size with min/max.
    //
    // CSS Flexible Box §9.7 "resolve the flexible lengths": distribute
    // the free space among the unfrozen flex items; any item whose
    // share violates its min/max is *frozen* at the clamped size and
    // the loop runs again over the survivors with the leftover budget,
    // until no clamp fires. A single pass with a per-item clamp (the
    // previous shape) left the clamped remainder unallocated — visible
    // as a gap — or, on the shrink side, as overflow past the container.
    //
    // Distribution inside a pass is rolling (Bresenham-style) so the
    // integer-division remainder is never dropped: two `Flex(1)`
    // children over 31 cells get 15 + 16, not 15 + 15.
    let mut final_main: Vec<u16> = child_info
        .iter()
        .map(|ci| match ci.main {
            MainNatural::Fixed(n) | MainNatural::Auto(n) => clamp_size(n, ci.min, ci.max),
            MainNatural::Flex(_) => 0,
        })
        .collect();
    {
        let mut frozen: Vec<bool> = child_info
            .iter()
            .map(|ci| !matches!(ci.main, MainNatural::Flex(_)))
            .collect();
        let mut budget: u32 = flex_remaining as u32;
        loop {
            let weight: u32 = child_info
                .iter()
                .zip(frozen.iter())
                .filter(|(_, f)| !**f)
                .map(|(ci, _)| match ci.main {
                    MainNatural::Flex(w) => w as u32,
                    _ => 0,
                })
                .sum();
            if weight == 0 {
                break;
            }
            // Every share in a pass is computed from the pass-start
            // budget; the budget consumed by items frozen in this pass is
            // subtracted only after the pass (a mid-pass subtraction made
            // later items' shares shrink and falsely froze them at their
            // floors).
            let pass_budget = budget;
            let mut frozen_this_pass: u32 = 0;
            let mut accumulated_weight: u32 = 0;
            let mut accumulated: u32 = 0;
            let mut clamped_any = false;
            for (i, ci) in child_info.iter().enumerate() {
                if frozen[i] {
                    continue;
                }
                let MainNatural::Flex(w) = ci.main else {
                    continue;
                };
                accumulated_weight = accumulated_weight.saturating_add(w as u32);
                let target = pass_budget
                    .saturating_mul(accumulated_weight)
                    .checked_div(weight)
                    .unwrap_or(0);
                let share = target.saturating_sub(accumulated).min(u16::MAX as u32) as u16;
                accumulated = target;
                let clamped = clamp_size(share, ci.min, ci.max);
                final_main[i] = clamped;
                if clamped != share {
                    // Freeze at the clamped size; its budget is spoken for.
                    frozen[i] = true;
                    frozen_this_pass = frozen_this_pass.saturating_add(clamped as u32);
                    clamped_any = true;
                }
            }
            if !clamped_any {
                break;
            }
            budget = budget.saturating_sub(frozen_this_pass);
        }
    }

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
    // Same §9.7 freeze loop as grow: an item that would shrink below
    // its min (explicit `min-width` or the auto-min of §4.5) is frozen
    // at the floor and the remaining overflow is redistributed over
    // the others. Bresenham accumulation keeps each pass exact.
    let net_budget = (main_budget as i32) - (gap_total as i32) + (overlap_savings as i32);
    if net_budget > 0 {
        let shrink_of = |ci: &ChildMain| -> u32 {
            dom.node(ci.id)
                .computed()
                .map(|c| c.flex_shrink as u32)
                .unwrap_or(1)
        };
        // Basis = the size before any shrinking in this loop.
        let basis: Vec<u16> = final_main.clone();
        let mut frozen: Vec<bool> = child_info.iter().map(|ci| shrink_of(ci) == 0).collect();
        let mut floors: Vec<Option<u16>> = vec![None; child_info.len()];
        loop {
            let total: i32 = final_main.iter().map(|&n| n as i32).sum();
            if total <= net_budget {
                break;
            }
            let overflow = (total - net_budget) as u32;
            let divisor: u32 = child_info
                .iter()
                .enumerate()
                .filter(|(i, _)| !frozen[*i])
                .map(|(i, ci)| (basis[i] as u32) * shrink_of(ci))
                .sum();
            let Some(divisor) = std::num::NonZeroU32::new(divisor) else {
                break; // nothing left that can shrink
            };
            let mut accumulated_basis: u32 = 0;
            let mut accumulated_shrink: u32 = 0;
            let mut clamped_any = false;
            for (i, ci) in child_info.iter().enumerate() {
                if frozen[i] {
                    continue;
                }
                accumulated_basis += (basis[i] as u32) * shrink_of(ci);
                let target_total_shrink =
                    ((accumulated_basis as u64 * overflow as u64) / divisor.get() as u64) as u32;
                let my_shrink = target_total_shrink.saturating_sub(accumulated_shrink) as u16;
                accumulated_shrink = target_total_shrink;
                // Honor min clamp — child can't shrink below its
                // `min-width` / `min-height`. Explicit `Cells(n)` is
                // stored in `ci.min`; the auto-min (implicit or
                // explicit `Auto`) is resolved lazily here per CSS
                // Flexbox §4.5 and cached per item.
                let floor = *floors[i].get_or_insert_with(|| {
                    ci.min.unwrap_or_else(|| {
                        resolve_auto_min(dom, ci.id, direction, main_budget, cross_budget)
                    })
                });
                let wanted = final_main[i].saturating_sub(my_shrink);
                if wanted < floor {
                    final_main[i] = floor;
                    frozen[i] = true;
                    clamped_any = true;
                } else {
                    final_main[i] = wanted;
                }
            }
            if !clamped_any {
                break;
            }
            // A clamp fired: the unfrozen items were shrunk against a
            // stale overflow figure. Restore them to their basis and
            // redistribute the recomputed overflow on the next pass.
            for i in 0..final_main.len() {
                if !frozen[i] {
                    final_main[i] = basis[i];
                }
            }
        }
    }

    // Position each child along main axis, scrolling by parent's
    // scroll offset.
    let scroll_main = parent_scroll(dom, children, direction);
    // `SCROLL-CROSS-AXIS-1`: the container's other scroll offset moves
    // every item along the cross axis (a column container scrolling
    // horizontally).
    let scroll_cross = parent_scroll(
        dom,
        children,
        match direction {
            Direction::Row => Direction::Column,
            Direction::Column => Direction::Row,
        },
    );

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
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

        // Resolve this child's main-axis start and end margins.
        // `Calc` was pre-resolved to `Cells` during `ChildMain`
        // construction above (see `resolve_margin`), so only the
        // `Cells | Auto` cases are reachable here.
        use crate::layout::MarginValue;
        let main_start_cells: i32 = match &child_info[i].main_start_margin {
            MarginValue::Cells(n) => i32::from(*n),
            MarginValue::Auto => i32::from(resolve_auto(&mut autos_consumed)),
            MarginValue::Calc(_) => unreachable!("Calc pre-resolved to Cells"),
        };
        let main_end_cells: i32 = match &child_info[i].main_end_margin {
            MarginValue::Cells(n) => i32::from(*n),
            MarginValue::Auto => i32::from(resolve_auto(&mut autos_consumed)),
            MarginValue::Calc(_) => unreachable!("Calc pre-resolved to Cells"),
        };
        main_cursor = main_cursor.saturating_add(main_start_cells);

        // Whether the child's main-axis size was declared `Auto` —
        // needed so `resolve_cross_size` knows whether to apply
        // aspect-ratio (which requires the main axis to be explicit).
        let main_was_auto = matches!(child_info[i].main, MainNatural::Auto(_));

        // Cross-axis margins (Flexbox §9.4): the item's outer cross size
        // includes them, so a stretched item shrinks by their sum and the
        // start margin offsets the box. `auto` cross margins take the
        // free cross space (both auto → centered), per §9.5.
        let cb_width = container.width;
        let (cross_start_m, cross_end_m) = match direction {
            Direction::Row => (&child_computed.margin.top, &child_computed.margin.bottom),
            Direction::Column => (&child_computed.margin.left, &child_computed.margin.right),
        };
        // Signed: a negative cross margin starts the box before the
        // container's edge and widens a stretched box (Flexbox §9.4).
        let cross_cells = |m: &crate::layout::MarginValue| -> i32 {
            if m.is_auto() {
                0
            } else {
                i32::from(m.resolve(cb_width))
            }
        };
        let cross_start_cells = cross_cells(cross_start_m);
        let cross_end_cells = cross_cells(cross_end_m);
        let cross_avail = (i32::from(cross_budget) - cross_start_cells - cross_end_cells)
            .clamp(0, i32::from(u16::MAX)) as u16;
        // Flexbox §9.5: an item with an `auto` cross margin is not
        // stretched — it takes its content size and the margins absorb
        // the free space.
        let stretch = !(cross_start_m.is_auto() || cross_end_m.is_auto());
        let cross_size = resolve_cross_size(
            dom,
            *child_id,
            &child_computed,
            cross_avail,
            container.width,
            direction,
            MainAxisFacts {
                size: *size,
                was_auto: main_was_auto,
                stretch,
            },
        );
        let cross_free = i32::from(cross_avail.saturating_sub(cross_size));
        let cross_offset: i32 = match (cross_start_m.is_auto(), cross_end_m.is_auto()) {
            (true, true) => cross_start_cells + cross_free / 2,
            (true, false) => cross_start_cells + cross_free,
            _ => cross_start_cells,
        };

        let child_rect = match direction {
            Direction::Row => LayoutRect::new(
                main_cursor,
                container.y + cross_offset - scroll_cross,
                *size,
                cross_size,
            ),
            Direction::Column => LayoutRect::new(
                container.x + cross_offset - scroll_cross,
                main_cursor,
                cross_size,
                *size,
            ),
        };

        layout_node(dom, *child_id, child_rect, container.width);

        // Advance cursor past this child + main-end margin + gap.
        main_cursor = main_cursor.saturating_add(*size as i32);
        main_cursor = main_cursor.saturating_add(main_end_cells);
        if i + 1 < child_list.len() {
            main_cursor = main_cursor.saturating_add(gap as i32);
            // Sibling-overlap pullback. Mirrors the gating in the
            // `overlap_savings` computation above: only fires when
            // gap == 0 AND parent has collapse AND both children
            // have a border on the shared edge. With gap > 0, the
            // gap is visible and the siblings don't overlap.
            if gap == 0
                && parent.border_collapse == crate::layout::BorderCollapse::Collapse
                && has_effective_border_on_edge(dom, child_info[i].id, edge_i)
                && has_effective_border_on_edge(dom, child_info[i + 1].id, edge_next)
            {
                main_cursor = main_cursor.saturating_sub(1);
            }
        }
    }
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
    /// Pre-resolved main-axis start margin. `Cells` carries the
    /// resolved cell count (including `calc()` percent terms folded
    /// against the parent's main-axis cb-width). `Auto` is preserved
    /// because the placement loop resolves auto margins lazily by
    /// distributing leftover free space.
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
            Some(Size::percent_of(main_budget as i32, *p).clamp(0, u16::MAX as i32) as u16)
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
    let cb_width = match direction {
        Direction::Row => main_budget,
        Direction::Column => cross_budget,
    };
    let content = content_min_size(dom, id, direction, cross_budget, cb_width);
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
/// What the cross-axis resolver needs to know about the main axis and
/// the item's margins.
struct MainAxisFacts {
    /// Resolved main-axis size (for `aspect-ratio`).
    size: u16,
    /// Whether the main size was declared `auto` (aspect-ratio needs an
    /// explicit main size).
    was_auto: bool,
    /// `false` when a cross margin is `auto` — the item is not stretched
    /// and takes its content size (Flexbox §9.5).
    stretch: bool,
}

fn resolve_cross_size(
    dom: &Dom<TuiExt>,
    child_id: NodeId,
    computed: &ComputedStyle,
    container_cross: u16,
    container_width: u16,
    direction: Direction,
    main: MainAxisFacts,
) -> u16 {
    let MainAxisFacts {
        size: main_size,
        was_auto: main_was_auto,
        stretch,
    } = main;
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
            Size::percent_of(container_cross as i32, *p).clamp(0, u16::MAX as i32) as u16
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
                intrinsic_size(dom, child_id, cross_dir, container_cross, container_width)
            } else if stretch {
                container_cross
            } else {
                // `auto` cross margin: content size, not stretch.
                intrinsic_size(dom, child_id, cross_dir, container_cross, container_width)
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
        Some(crate::layout::MinSize::Auto) => Some(intrinsic_size(
            dom,
            child_id,
            cross_dir,
            container_cross,
            container_width,
        )),
    };
    clamp_size(natural, min, max)
}
