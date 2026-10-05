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
//! 4. `Size::Flex(w)` (rdom's `fr`) → a flex base size of 0 growing
//!    by `w` (when `flex-grow` is 0).
//!
//! `flex-basis` (`auto` → the main size above) gives the flex base
//! size; `flex-grow` / `flex-shrink` resolve the flexible lengths (§9.7,
//! `distribute`).
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
//!   threads the flex lines through the pieces below in spec order.
//! - [`lines`] — §9.3 line breaking, per-line §9.7, line cross sizes
//!   and `align-content: normal`, and a multi-line container's
//!   intrinsic cross size (§9.9).
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
mod distribute;
mod justify;
mod lines;
mod main_axis;
mod placement;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect};
use crate::render::inline::compute_inline_layout;
use crate::style::ComputedStyle;

use super::ifc::is_ifc_block;
use super::margin_trim::FlexTrim;
use super::{element_children_of, layout_node};
use collapse::SiblingOverlap;
use cross::CrossSpace;
pub(in crate::render::layout_pass) use lines::{is_multi_line, lines_cross_size};
use main_axis::{MainBudgets, collect_main_axis_items};
use placement::{FlexLine, place_items};

/// `visibility: collapse` on a flex item (Flexbox §4.4): it is laid out
/// as a strut — no main size, its cross size kept — and drawn as
/// `hidden`. The document root's children are flex items of rdom's
/// viewport column only as a layout device (DIVERGENCES), as a
/// browser's `<body>` children are blocks: there `collapse` is
/// `hidden` (CSS Display 3 §4).
pub(in crate::render::layout_pass) fn is_collapsed(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::node::TuiNodeExt;
    dom.node(id)
        .computed()
        .is_some_and(|c| c.visibility == crate::layout::Visibility::Collapse)
        && crate::render::box_tree::box_parent(dom, id).is_some_and(|p| {
            let parent = dom.node(p);
            parent.node_type() == rdom_core::NodeType::Element
                && parent
                    .computed()
                    .is_some_and(|c| c.flow == crate::layout::Flow::Flex)
        })
}

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
        // The lines sit in the *scrolled* content rect, as paint and
        // hit-test read them back.
        let lines_at = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(container);
        super::positioning::record_static_positions_in_ifc(dom, id, &inline_layout, lines_at);
        // Atomic inline-block fragments (`<button>` in
        // `<p>hi <button>X</button> ok</p>`) need their layout rect
        // written so hit-test descends into them, and need
        // `layout_node` recursion so their own subtrees lay out
        // (text wrap, pseudos, descendants). Snapshot fragments
        // first to satisfy the borrow checker.
        let atoms = crate::render::inline::atomic_placements(&inline_layout, lines_at);
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
        crate::layout::Flow::Block | crate::layout::Flow::FlowRoot => {
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
    let mut children: Vec<NodeId> = element_children_of(dom, id)
        .into_iter()
        .filter(|&c| super::is_in_flow(dom, c))
        .collect();
    // CSS Flexbox §5.4: the items are laid out in order-modified
    // document order.
    crate::render::box_tree::sort_by_order(dom, &mut children);
    // `D-M2-2`: a positioned child's static position in a flex
    // container is the content box's start — Flexbox §4.1 places it as
    // the sole item; `justify-content` / `align-items` are not applied
    // (DIVERGENCES). Both scroll offsets apply, as they do to the
    // in-flow items.
    let (static_x, static_y) = dom.node(id).ext().map_or((container.x, container.y), |e| {
        (container.x - e.scroll_x, container.y - e.scroll_y)
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

/// Lay out a flex container's items: `children` (already filtered to
/// in-flow items, in order-modified document order) inside `container`,
/// driven by `parent`'s `flex-direction`, `flex-wrap`, gaps and
/// `border-collapse`.
///
/// Runs the CSS Flexible Box algorithm in spec order — gather the flex
/// base sizes (§9.2), collect the items into lines (§9.3, `lines`),
/// resolve each line's flexible lengths and `auto` margins (§9.7 /
/// §9.5), size the lines on the cross axis (§9.4 steps 7–8 and 15),
/// then place each line's items along the main axis and size them on
/// the cross axis (§9.4–§9.6) — with the per-concern math in the sibling
/// modules.
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
    let cross_direction = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    // CSS Box Alignment 3 §8.1: the gutters between items on a line, and
    // between the lines.
    let gap = super::resolve_gap(parent, container, direction);
    let line_gap = super::resolve_gap(parent, container, cross_direction);

    let container = collapse::inset_container_for_children(dom, children, parent, container);

    // The container's inner size on the main and cross axes.
    let (main_budget, cross_budget) = match direction {
        Direction::Row => (container.width, container.height),
        Direction::Column => (container.height, container.width),
    };
    let budgets = MainBudgets {
        main: main_budget,
        cross: cross_budget,
    };

    let trim = FlexTrim::of(parent, direction);
    // CSS Writing Modes 4 §2.1: under `rtl` the inline axis — a row's
    // main axis, a column's cross axis — runs right to left; CSS Flexbox
    // §5.1: `row-reverse` / `column-reverse` swap the main axis's start
    // and end (so a `row-reverse` under `rtl` runs left to right), and
    // §5.2: `wrap-reverse` swaps the cross axis's. A mirrored axis is
    // laid out in a mirrored frame (the item's physical end margin its
    // start one) and flipped back across the container.
    let flip = AxisFlip::of(parent, direction);
    let mut items = collect_main_axis_items(dom, children, direction, budgets, trim, flip.main);

    // §9.3: a single-line container's one line holds every item.
    let multi_line = lines::is_multi_line(parent);
    let line_ranges = if multi_line {
        lines::break_lines(dom, &items, direction, budgets, gap)
    } else {
        std::iter::once(0..items.len()).collect()
    };
    let overlap = SiblingOverlap::new(parent, gap, direction);

    // §9.7 per line, then each line's cross size: a single line is the
    // container's inner cross size (§9.4 step 8); a multi-line
    // container's lines are as large as their largest outer
    // hypothetical cross size, stretched by `align-content: normal`
    // (§9.4 step 15).
    let last_line = line_ranges.len() - 1;
    let line_trim = |k: usize| FlexTrim {
        cross_start: trim.cross_start && (!multi_line || k == 0),
        cross_end: trim.cross_end && (!multi_line || k == last_line),
        ..trim
    };
    let mut resolved = Vec::with_capacity(line_ranges.len());
    let mut line_cross = Vec::with_capacity(line_ranges.len());
    for (k, range) in line_ranges.iter().enumerate() {
        if multi_line {
            lines::trim_line_edges(&mut items[range.clone()], trim);
        }
        let line = lines::resolve_line_main(
            dom,
            &items[range.clone()],
            &children[range.clone()],
            direction,
            budgets,
            gap,
            &overlap,
        );
        line_cross.push(if multi_line {
            let t = line_trim(k);
            items[range.clone()]
                .iter()
                .zip(&line.final_main)
                .map(|(ci, &size)| {
                    cross::hypothetical_outer_cross(
                        dom,
                        ci.id,
                        container.width,
                        CrossSpace {
                            line: cross_budget,
                            container: Some(cross_budget),
                        },
                        direction,
                        cross::ResolvedMain {
                            size,
                            was_auto: ci.main_auto,
                            trim_cross_start: t.cross_start,
                            trim_cross_end: t.cross_end,
                            mirror: flip.cross,
                        },
                    )
                })
                .max()
                .unwrap_or(0)
        } else {
            cross_budget
        });
        resolved.push(line);
    }
    if multi_line {
        lines::stretch_lines(&mut line_cross, cross_budget, line_gap);
    }

    let mut line_offset: i32 = 0;
    for (k, (range, line)) in line_ranges.iter().zip(resolved).enumerate() {
        // §8.2: `justify-content` places the line's leftover free space.
        let justify = justify::justify_offsets(
            parent,
            direction,
            flip.main,
            line.free,
            range.len(),
            line.has_auto_margins,
        );
        place_items(
            dom,
            &children[range.clone()],
            FlexLine {
                items: &items[range.clone()],
                final_main: &line.final_main,
                justify: &justify,
                container,
                direction,
                gap,
                space: CrossSpace {
                    line: line_cross[k],
                    container: Some(cross_budget),
                },
                line_offset,
                auto_margins: line.auto_margins,
                overlap,
                trim: line_trim(k),
                flip,
            },
        );
        line_offset += i32::from(line_cross[k]) + i32::from(line_gap);
    }
}

/// Which of a flex container's axes run from their physical end edge
/// — right to left, or bottom to top.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AxisFlip {
    /// The main axis: a row's under `rtl` XOR `row-reverse`, a column's
    /// under `column-reverse`.
    pub(super) main: bool,
    /// The cross axis: a column's under `rtl` XOR `wrap-reverse`, a
    /// row's under `wrap-reverse` (CSS Flexbox §5.2: cross-start and
    /// cross-end swap; a row's cross axis, the block axis, otherwise runs
    /// top to bottom).
    pub(super) cross: bool,
}

impl AxisFlip {
    pub(super) fn of(container: &ComputedStyle, direction: Direction) -> Self {
        let rtl = super::margin_trim::inline_reversed(container);
        let reverse = container.flex_reverse;
        let wrap_reverse = container.flex_wrap == crate::layout::FlexWrap::WrapReverse;
        match direction {
            Direction::Row => Self {
                main: rtl != reverse,
                cross: wrap_reverse,
            },
            Direction::Column => Self {
                main: reverse,
                cross: rtl != wrap_reverse,
            },
        }
    }
}
