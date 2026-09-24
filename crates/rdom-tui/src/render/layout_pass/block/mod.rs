//! Block layout pass — CSS 2.1 §10 normal flow.
//!
//! Given a block container (`flow: Block`) and its in-flow children,
//! stacks the children vertically in document order at their natural
//! heights. No distribution, no shrink-to-fit; container overflows
//! below its content box if children don't fit.
//!
//! Width resolution follows CSS 2.1 §10.3.3 — the seven-term sum
//! `margin-left + border-left + padding-left + width + padding-right
//! + border-right + margin-right` must equal the containing-block
//! width. Auto margins absorb leftover horizontal space (the
//! `margin: 0 auto` centering pattern).
//!
//! Height: each child takes its declared `height` (`Fixed`), its
//! resolved percentage (`Percent`), or its intrinsic content height
//! (`Auto`). Min/max clamping applies after computing the size.
//!
//! **Scope (BFC-1 through phase 4):**
//! - Width formula + auto margins + min/max clamp (phase 2).
//! - Plain vertical stacking (no margin collapse — phase 5).
//! - Anonymous box generation around inline-level children (phase 3),
//!   including atomic inline-block packing (phase 3.5b).
//! - Live dispatch from `layout_children` via cascaded `Flow::Block`
//!   (phase 4.1); border-collapse parent-edge inset + scroll cursor
//!   offset mirror flex behavior so the two modes agree.
//! - Strict percent-height-needs-definite-parent — phase 6 will
//!   tighten this; for now percent resolves against the container.
//!
//! ## Module layout
//!
//! - `mod.rs` — run partitioning, anonymous block boxes and the placement loop.
//! - [`margin_collapse`] — §8.3.1 accumulator, predicates and chain walkers.
//! - [`width`] — §10.3.3 width / horizontal margin resolution.
//! - [`height`] — §10.5 / §10.6.3 height resolution.

mod height;
mod margin_collapse;
mod width;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout_for_run;
use crate::style::ComputedStyle;

use super::is_in_flow;
use super::layout_node;
pub(super) use height::nearest_block_ancestor_height_is_definite;
use height::resolve_block_height;
use margin_collapse::{
    MarginAccumulator, is_empty_collapse_through, outer_bottom_margin, outer_top_margin,
    parent_collapses_bottom_with_last_child, parent_collapses_top_with_first_child,
    store_margin_chain_memo,
};
use width::resolve_block_width;

/// Lay out `id`'s in-flow children per CSS 2.1 §10. Partitions
/// children into runs of consecutive block-level vs inline-level
/// nodes; block runs get individual block layout; inline runs
/// fold into **anonymous block boxes** (CSS 2.1 §9.2.1.1) that
/// each establish their own IFC.
///
/// Stores anonymous boxes on the parent's `TuiExt.anonymous_blocks`
/// — paint / hit-test / selection iterate this Vec alongside the
/// singular `inline_layout` field.
/// Returned by [`layout_block_children`] so the caller (`layout_node`)
/// can resolve an `Auto` parent height against the actual content
/// extent. Captures the margin-collapse-aware measurement that
/// `intrinsic_size`'s flat sum can't see.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BlockMeasurement {
    /// Top-of-content to bottom-of-last-block, including any
    /// trailing bottom margin that the parent *traps* (i.e. won't
    /// escape upward via parent-last-child collapse). For BFC
    /// containers (`overflow: hidden`, flex, abs-pos, …) this
    /// includes leading and trailing margins both — the BFC seals
    /// them in. For collapse-eligible parents, leading/trailing
    /// margins escape upward and aren't counted here; the
    /// grandparent picks them up via the `accumulate_outer_*`
    /// helpers.
    pub content_height: u16,
}

pub(super) fn layout_block_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    parent_computed: &ComputedStyle,
) -> BlockMeasurement {
    // Collect ALL direct child nodes (text + element). Block layout
    // distinguishes inline-level (text + Display::Inline/InlineBlock
    // elements) from block-level (Display::Block elements) — text
    // nodes are inline-level participants in an anonymous block per
    // CSS 2.1 §9.2.1.1 rule 2.
    let raw_children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    if raw_children.is_empty() {
        return BlockMeasurement::default();
    }

    // Filter out-of-flow elements; text nodes are always in flow.
    // The `child_range` indices below are into THIS filtered list.
    let in_flow: Vec<(usize, NodeId)> = raw_children
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, c)| is_in_flow(dom, *c))
        .collect();
    // `D-M2-2`: out-of-flow positioned children take their static
    // position (CSS 2.1 §10.3.7 / §10.6.4) from the flow cursor at the
    // point where their hypothetical box would have gone — recorded
    // just before the in-flow sibling that follows them is placed.
    let (static_before, static_trailing) = super::positioning::static_anchors(dom, &raw_children);
    if in_flow.is_empty() {
        // Clear any stale anonymous boxes from a previous layout —
        // matches flex's `ext.inline_layout = None` reset.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.anonymous_blocks.clear();
        }
        let (scroll_x, scroll_y) = dom
            .node(id)
            .ext()
            .map_or((0, 0), |e| (e.scroll_x as i32, e.scroll_y as i32));
        for &n in &static_trailing {
            super::positioning::record_static_position(
                dom,
                n,
                container.x - scroll_x,
                container.y - scroll_y,
            );
        }
        return BlockMeasurement::default();
    }

    // Partition into runs. A run is a contiguous sequence of in-flow
    // children that share a level (block or inline). When the level
    // flips, the run closes and a new one opens. Comments and
    // fragments are treated as inline-level (no effect on layout
    // beyond breaking adjacency).
    let mut runs: Vec<Run> = Vec::new();
    for (orig_idx, child_id) in &in_flow {
        let kind = child_level(dom, *child_id);
        match runs.last_mut() {
            Some(last) if last.kind == kind => {
                last.children.push(*child_id);
                last.child_range.1 = orig_idx + 1;
            }
            _ => runs.push(Run {
                kind,
                children: vec![*child_id],
                child_range: (*orig_idx, orig_idx + 1),
            }),
        }
    }

    // Parent-child border-collapse inset (CSS 2.1 §17.6.3 +
    // BFC-1 invariant): when this container is `border-collapse:
    // collapse` with its own border, `layout_node` already expanded
    // its content area to extend into the border ring. That's
    // correct ONLY when the first/last child has its own border to
    // share the cell with. Content-bearing children (no border)
    // would land on the parent's painted border row. Apply the
    // same per-edge inset flex uses so the two layout modes agree.
    let in_flow_ids: Vec<NodeId> = in_flow.iter().map(|(_, id)| *id).collect();
    let (top_inset, bot_inset, left_inset, right_inset) =
        super::border_collapse::collapse_parent_edge_insets(dom, &in_flow_ids, parent_computed);
    let container = LayoutRect::new(
        container.x + left_inset as i32,
        container.y + top_inset as i32,
        container.width.saturating_sub(left_inset + right_inset),
        container.height.saturating_sub(top_inset + bot_inset),
    );

    let containing_block_width = container.width;
    // Apply this container's scroll_y to the starting cursor (mirrors
    // `flex::layout_flex_children`'s `container.y - scroll_main`). The
    // scroll itself lives on the parent's `ext.scroll_y`; the flex pass
    // reads it via the *children's* `parent_scroll` helper, which we
    // reuse here so block and flex agree on the offset.
    let scroll_y = super::parent_scroll(dom, &in_flow_ids, crate::layout::Direction::Column);
    // `SCROLL-CROSS-AXIS-1`: horizontal scroll shifts every box left.
    let scroll_x = super::parent_scroll(dom, &in_flow_ids, crate::layout::Direction::Row);
    let content_x = container.x - scroll_x;
    let mut y_cursor: i32 = container.y - scroll_y;
    let mut anon_blocks: Vec<AnonymousIfc> = Vec::new();

    // CSS 2.1 §8.3.1 — vertical margin collapse accumulator.
    // Tracks the unresolved set of margins between the last placed
    // block (or the container's top) and the next block to be
    // placed. Adjacent in-flow block siblings' vertical margins
    // collapse into one: `max(positives) + min(negatives)`.
    //
    // Anonymous block boxes (inline runs in mixed content) have
    // zero margins so they participate transparently — they don't
    // contribute to the accumulator but they also don't reset it
    // wholesale when surrounded by block siblings. Out-of-flow
    // siblings are already filtered out of `in_flow`.
    //
    // Parent–first-child and parent–last-child collapse (Phase 5.2)
    // + empty-block collapse-through (Phase 5.3) build on this same
    // accumulator.
    let mut margin_acc = MarginAccumulator::new();

    // Phase 5.2 — parent–first-child top margin collapse.
    // When the parent has no top padding, no top border, and doesn't
    // establish a new BFC, the first in-flow block child's
    // `margin-top` collapses through the parent. The merged margin
    // ideally surfaces at the parent's OUTER top (the
    // parent's parent should see it). For now we implement the
    // local half: suppress the first child's `margin-top` so it
    // doesn't create extra space inside the parent's content area.
    // The upward-merge half is tracked as known incompleteness in
    // [[bfc1-margin-collapse-upward-propagation]] (`TECH_DEBT.md`).
    let suppress_first_top_margin = parent_collapses_top_with_first_child(parent_computed);
    let suppress_last_bottom_margin = parent_collapses_bottom_with_last_child(parent_computed);
    let last_block_run_idx = runs
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, r)| (r.kind == RunKind::Block).then_some(i));

    // CSS3 Box Alignment Module — `row-gap` applies between
    // adjacent in-flow **block-level element children**. We
    // deliberately don't insert gap around anonymous block boxes
    // wrapping inline-only runs (whitespace text between block
    // siblings produces 0-height anons; counting them as gap
    // boundaries would multiply gaps unexpectedly).
    let row_gap = super::resolve_gap(parent_computed, container, Direction::Column);

    let mut placed_block_count: usize = 0;
    // BORDER-MODEL-1 (M6): track the previous direct block sibling
    // so we can apply the sibling-overlap pullback when both this
    // block and its predecessor have a visible border on the shared
    // edge under `border-collapse: collapse`. Mirrors flex.rs's
    // pullback. The variable resets to `None` when an inline-run
    // anonymous block intervenes — its non-zero height breaks
    // border-adjacency, so border-overlap can't apply across it.
    let mut prev_block_id: Option<NodeId> = None;
    for (run_idx, run) in runs.iter().enumerate() {
        match run.kind {
            RunKind::Block => {
                let is_last_block_run = Some(run_idx) == last_block_run_idx;
                let last_child_idx = run.children.len() - 1;
                for (i, &child) in run.children.iter().enumerate() {
                    if let Some(oof) = static_before.get(&child) {
                        // The hypothetical box has zero margins: it
                        // collapses through whatever is buffered.
                        let y = y_cursor + i32::from(margin_acc.resolved());
                        for &n in oof {
                            super::positioning::record_static_position(dom, n, content_x, y);
                        }
                    }
                    let is_first_block_placed = placed_block_count == 0;
                    let is_last_block_placed = is_last_block_run && i == last_child_idx;
                    if !is_first_block_placed && row_gap > 0 {
                        // Gap between adjacent block-level element
                        // children. Margins collapse normally above;
                        // gap is added on top per CSS3 Box Alignment.
                        y_cursor += row_gap as i32;
                    }
                    // BORDER-MODEL-1 (M6) block-flow sibling overlap:
                    // same rule as flex.rs — parent has `collapse`,
                    // row-gap is 0, AND both this child and the
                    // previous block sibling have a visible border on
                    // the shared (top / bottom) edge. Pull the cursor
                    // back by 1 so the borders coincide and paint-time
                    // mask-OR produces the junction glyph.
                    if row_gap == 0
                        && parent_computed.border_collapse
                            == crate::layout::BorderCollapse::Collapse
                        && let Some(prev) = prev_block_id
                    {
                        // Any non-None border (including Hidden) counts
                        // for sibling-overlap participation, mirroring
                        // flex.rs's `has_effective_border_on_edge`. Hidden
                        // suppresses paint at the shared cell but still
                        // participates in layout so the shared cell
                        // exists for the kill-switch to suppress.
                        let prev_bot = dom
                            .node(prev)
                            .computed()
                            .map(|c| !c.border.bottom.is_none())
                            .unwrap_or(false);
                        let curr_top = dom
                            .node(child)
                            .computed()
                            .map(|c| !c.border.top.is_none())
                            .unwrap_or(false);
                        if prev_bot && curr_top {
                            y_cursor -= 1;
                        }
                    }
                    y_cursor = lay_out_block_child(
                        dom,
                        child,
                        BlockPlace {
                            container: LayoutRect::new(
                                content_x,
                                container.y,
                                container.width,
                                container.height,
                            ),
                            containing_block_width,
                            y_cursor,
                            margin_acc: &mut margin_acc,
                            suppress_top_margin: is_first_block_placed && suppress_first_top_margin,
                            suppress_bottom_margin: is_last_block_placed
                                && suppress_last_bottom_margin,
                        },
                    );
                    placed_block_count += 1;
                    prev_block_id = Some(child);
                }
            }
            RunKind::Inline => {
                // Anonymous block box wrapping this inline run. Its
                // IFC packs the run's children at the container's
                // content width. Height = packed line count.
                //
                // Resolve any accumulated margin from the previous
                // block sibling before placing the anon box. Anon
                // boxes themselves contribute zero margins (no CSS
                // identity), so the accumulator empties after this
                // placement — the next block starts a fresh
                // accumulator.
                let inline_layout =
                    compute_inline_layout_for_run(dom, id, &run.children, containing_block_width);
                let height = inline_layout.height();
                let resolved_gap = margin_acc.resolved();
                margin_acc = MarginAccumulator::new();
                let anon_y = y_cursor + resolved_gap as i32;
                let rect = LayoutRect::new(content_x, anon_y, containing_block_width, height);
                // Layout atomic inline-block children at their
                // fragment rects. This both writes their layout
                // rects (so hit-test descends into them — e.g.
                // `<form><button>Go</button></form>` button clicks
                // route to the button, not the form) and recurses
                // into their subtrees (so `<button>`'s own inner
                // text-only layout, pseudos, etc. get computed).
                layout_atomic_inline_blocks(dom, &inline_layout, rect);
                for c in &run.children {
                    if let Some(oof) = static_before.get(c) {
                        for &n in oof {
                            let (x, y) = super::positioning::static_position_in_ifc(
                                dom,
                                id,
                                n,
                                &inline_layout,
                                rect,
                            );
                            super::positioning::record_static_position(dom, n, x, y);
                        }
                    }
                }
                anon_blocks.push(AnonymousIfc {
                    rect,
                    inline_layout,
                    child_range: run.child_range,
                });
                y_cursor = anon_y + height as i32;
                // BORDER-MODEL-1 (M6): an inline-run anon block breaks
                // block-to-block border adjacency. Reset the
                // overlap-tracker so the next block sibling is treated
                // as the start of a fresh adjacency chain.
                prev_block_id = None;
            }
        }
    }

    // Positioned children after the last in-flow child: continue the
    // last inline run, or sit below the last block and the margin that
    // collapses with their zero-margin hypothetical box (CSS 2.1
    // §8.3.1: as if the box had a bottom border) — including a bottom
    // margin that escaped through the parent and so never reached the
    // accumulator.
    let last_run_is_inline = matches!(runs.last().map(|r| r.kind), Some(RunKind::Inline));
    if !static_trailing.is_empty() {
        let mut trailing_margin = margin_acc;
        if suppress_last_bottom_margin
            && !last_run_is_inline
            && let Some(&last) = runs.last().and_then(|r| r.children.last())
        {
            let last_computed = dom
                .node(last)
                .computed_rc()
                .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
            let mut memo = Vec::new();
            trailing_margin.merge(outer_bottom_margin(
                dom,
                last,
                &last_computed,
                containing_block_width,
                &mut memo,
            ));
        }
        let below_last_block = y_cursor + i32::from(trailing_margin.resolved());
        for &n in &static_trailing {
            let (x, y) = match anon_blocks.last() {
                Some(anon) if last_run_is_inline => super::positioning::static_position_in_ifc(
                    dom,
                    id,
                    n,
                    &anon.inline_layout,
                    anon.rect,
                ),
                _ => (content_x, below_last_block),
            };
            super::positioning::record_static_position(dom, n, x, y);
        }
    }

    // Write anon boxes to the parent. Empty Vec is the normal state
    // for pure-block containers — clears any stale entries from a
    // previous layout where the tree may have had different shape.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.anonymous_blocks = anon_blocks;
    }

    // CSS 2.1 §10.6.3 — content height measurement. `y_cursor` is
    // the bottom of the last placed in-flow content; subtract the
    // initial cursor (`container.y - scroll_y`) to get the extent.
    // Any unresolved bottom margin in the accumulator escapes
    // upward through parent-last-child collapse (handled by the
    // grandparent's `accumulate_outer_bottom_margin`) — UNLESS the
    // parent establishes a new BFC, in which case the margin is
    // trapped inside this container's height.
    let initial_cursor = container.y - scroll_y;
    let mut content_height = (y_cursor - initial_cursor).max(0);
    if !suppress_last_bottom_margin {
        // Margin doesn't escape upward — fold the running
        // accumulator into the measured height. (Trailing positive
        // margins contribute to height; negative pull content up.)
        content_height = (content_height + margin_acc.resolved() as i32).max(0);
    }
    BlockMeasurement {
        content_height: content_height.min(u16::MAX as i32) as u16,
    }
}

/// Recurse layout into atomic inline-block fragments — write each
/// atom's layout rect (so hit-test descends) and call `layout_node`
/// to lay out the atom's own subtree (its own inner inline_layout,
/// pseudo positioning, descendants).
///
/// `anon_rect` is the anonymous block box's rect (or the singular
/// IFC's content rect). Fragment x is offset from `anon_rect.x`;
/// fragment line index gives the y row.
fn layout_atomic_inline_blocks(
    dom: &mut Dom<TuiExt>,
    inline_layout: &crate::render::inline::InlineLayout,
    anon_rect: LayoutRect,
) {
    for (id, rect) in crate::render::inline::atomic_placements(inline_layout, anon_rect) {
        layout_node(dom, id, rect, anon_rect.width);
    }
}

/// Lay out a single block-level child. Folds the child's `margin-top`
/// into the running margin accumulator, resolves the accumulator into
/// a single gap above the child, places the child, then primes the
/// accumulator with the child's `margin-bottom` for the next sibling.
///
/// Returns the new y cursor — the bottom edge of the child's outer
/// rect (NOT including its bottom margin, which is now buffered in
/// `margin_acc`). The container's own height computation (Phase 6) and
/// the parent-last-child collapse (Phase 5.2) consume the leftover
/// accumulator separately.
/// Per-child placement context — bundles the in-flow positioning
/// state so `lay_out_block_child`'s signature stays narrow.
struct BlockPlace<'a> {
    container: LayoutRect,
    containing_block_width: u16,
    y_cursor: i32,
    margin_acc: &'a mut MarginAccumulator,
    /// Phase 5.2 — first-block-in-this-container + parent's
    /// `parent_collapses_top_with_first_child`. When true, the
    /// child's `margin-top` is suppressed to model parent-first-
    /// child collapse.
    suppress_top_margin: bool,
    /// Symmetric to `suppress_top_margin` — for the last in-flow
    /// block child + parent's `parent_collapses_bottom_with_last_child`.
    suppress_bottom_margin: bool,
}

fn lay_out_block_child(dom: &mut Dom<TuiExt>, child: NodeId, ctx: BlockPlace<'_>) -> i32 {
    let BlockPlace {
        container,
        containing_block_width,
        y_cursor,
        margin_acc,
        suppress_top_margin,
        suppress_bottom_margin,
    } = ctx;
    let computed = dom
        .node(child)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

    let resolved = resolve_block_width(&computed, containing_block_width);
    let height = resolve_block_height(
        dom,
        child,
        &computed,
        resolved.width,
        container.height,
        containing_block_width,
    );

    // Phase 5.2 + 5.4 — fold this child's *outer top* margin into
    // the accumulator. `accumulate_outer_top_margin` walks the
    // collapse-eligible chain (this child, its first block child
    // if they collapse, that one's first block child, …) so the
    // grandparent / great-grandparent sees the merged margin
    // surfacing at this block's outer top edge per CSS 2.1 §8.3.1.
    // Closes `BFC1-MARGIN-COLLAPSE-UPWARD-1`.
    //
    // When `suppress_top_margin` is set, this child's top margin
    // already escaped upward via the parent's call to this function
    // — contribute nothing here.
    let mut memo = Vec::new();
    if !suppress_top_margin {
        margin_acc.merge(outer_top_margin(
            dom,
            child,
            &computed,
            containing_block_width,
            &mut memo,
        ));
    }

    // Symmetric: compute the outer bottom margin (chain through
    // last collapse-eligible block descendant) so the NEXT sibling
    // sees the merged value, not just the raw `margin-bottom`. When
    // `suppress_bottom_margin` is set, the bottom already escaped
    // upward (parent-last-child collapse).
    let mut outer_bottom = MarginAccumulator::new();
    if !suppress_bottom_margin {
        outer_bottom =
            outer_bottom_margin(dom, child, &computed, containing_block_width, &mut memo);
    }
    store_margin_chain_memo(dom, &memo);

    // Phase 5.3 — empty-block collapse-through. A block with no
    // content, no padding, no border, and zero height has its top
    // + bottom margins meet — they fold into the surrounding
    // accumulator together rather than resetting it.
    let collapse_through = is_empty_collapse_through(dom, child, &computed, height);

    let outer_x = container.x + resolved.margin_left as i32;
    // The "gap" used for placement: for collapse-through children we
    // still place the empty box visually at the resolved-so-far
    // position (mostly for downstream layouts that ask for its
    // rect), but we do NOT advance the y_cursor or reset the
    // accumulator — the next sibling's gap will collapse with
    // everything accumulated so far.
    let gap = margin_acc.resolved();
    let outer_y = y_cursor + gap as i32;
    let outer_rect = LayoutRect::new(outer_x, outer_y, resolved.width, height);
    layout_node(dom, child, outer_rect, containing_block_width);

    // `layout_node` finalizes an `Auto` height via CSS 2.1 §10.6.3
    // content measurement, which can exceed the pre-layout
    // `resolve_block_height` estimate — notably for a mixed-content
    // block (a text run + a block child), whose `intrinsic_size`
    // walk counts element children only and so misses the text
    // run's anonymous-block row. Advance the cursor by the child's
    // ACTUAL laid-out height so the next sibling can't overlap it.
    // Use the intended `outer_y` (not the written `rect.y`, which
    // may carry a `position: relative` shift that must NOT move
    // siblings).
    let actual_height = dom
        .node(child)
        .layout_rect()
        .map(|r| r.height)
        .unwrap_or(height);

    if collapse_through {
        // Fold the outer bottom into the SAME accumulator and
        // leave y_cursor where it was. Next sibling's `gap`
        // computation will see all of A.mb, E.mt, E.mb, B.mt.
        margin_acc.merge(outer_bottom);
        y_cursor
    } else {
        // Normal block: advance y_cursor past the child and reset
        // the accumulator to just this child's outer bottom margin.
        *margin_acc = outer_bottom;
        outer_y + actual_height as i32
    }
}

/// One run of consecutive children sharing a level (block-level or
/// inline-level). Block runs get per-child block layout; inline
/// runs fold into one anonymous block per CSS 2.1 §9.2.1.1.
struct Run {
    kind: RunKind,
    /// Direct-child NodeIds in document order.
    children: Vec<NodeId>,
    /// Indices into the parent's raw `child_nodes()` order, as
    /// `[start, end)`. Stored on the resulting `AnonymousIfc` so
    /// paint / hit-test can map back to surrounding context.
    child_range: (usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Block,
    Inline,
}

/// Classify a direct child as block-level vs inline-level. Text
/// nodes are always inline-level; element children depend on their
/// `Display`. Per CSS 2.1 §9.2: only `Block` elements are
/// block-level; `Inline` and `InlineBlock` are inline-level (the
/// inline-block participates in IFC as an atomic box per phase
/// 3.5's planned inline-block-in-IFC packing).
fn child_level(dom: &Dom<TuiExt>, id: NodeId) -> RunKind {
    let node = dom.node(id);
    match node.node_type() {
        NodeType::Text => RunKind::Inline,
        NodeType::Element => {
            let display = node
                .ext()
                .and_then(|e| e.computed.as_ref())
                .map(|c| c.display)
                .unwrap_or(crate::layout::Display::Block);
            match display {
                crate::layout::Display::Inline | crate::layout::Display::InlineBlock => {
                    RunKind::Inline
                }
                crate::layout::Display::Block | crate::layout::Display::None => RunKind::Block,
            }
        }
        // Comments, fragments — treat as inline-level (effectively
        // invisible; they don't break runs).
        _ => RunKind::Inline,
    }
}
