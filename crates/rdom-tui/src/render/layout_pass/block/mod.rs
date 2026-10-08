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
//! - Vertical stacking with §8.3.1 margin collapsing (`margin_collapse`).
//! - Anonymous box generation around inline-level children (phase 3),
//!   including atomic inline-block packing (phase 3.5b), and a box of
//!   its own for a `::before` / `::after` beside a block-level edge
//!   child (CSS 2.1 §9.2.1.1; placement rules in
//!   `render::inline::generated`).
//! - Live dispatch from `layout_children` via cascaded `Flow::Block`
//!   (phase 4.1); border-collapse parent-edge inset + scroll cursor
//!   offset mirror flex behavior so the two modes agree.
//! - Percent heights resolve only against a definite containing block
//!   (`height`).
//!
//! ## Module layout
//!
//! - `mod.rs` — run partitioning, anonymous block boxes and the placement loop.
//! - [`margin_collapse`] — §8.3.1 accumulator, predicates and chain walkers.
//! - [`width`] — §10.3.3 width / horizontal margin resolution.
//! - [`height`] — §10.5 / §10.6.3 height resolution.
//! - [`place`] — placing one block-level child.
//! - [`inline_run`] — an inline run's anonymous block box.
//! - [`runs`] — block-level / inline-level run partitioning.

#[cfg(test)]
thread_local! {
    /// Box sequences built to decide a first child's clearance (cost
    /// tests, `margin_collapse::first_child_has_clearance`).
    pub(in crate::render::layout_pass) static CLEARANCE_SCANS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

mod align;
mod flow;
pub(in crate::render::layout_pass) mod generated;
mod height;
mod inline_run;
mod margin_collapse;
pub(in crate::render::layout_pass) mod measure;
mod place;
mod runs;
mod width;

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::render::inline::RunPseudos;
use crate::style::ComputedStyle;

use super::is_in_flow;
use super::layout_node;
pub(super) use align::{align_content_lead, aligns, justify_offset};
use height::resolve_block_height;
pub(super) use height::{height_is_definite_below, nearest_block_ancestor_height_is_definite};
#[cfg(debug_assertions)]
pub(super) use margin_collapse::debug_assert_no_margin_chain_memo;
pub(super) use margin_collapse::establishes_independent_formatting_context as establishes_bfc;
use margin_collapse::{
    MarginAccumulator, is_empty_collapse_through, outer_bottom_margin, outer_top_margin,
    store_margin_chain_memo,
};
use place::BlockPlace;
#[cfg(test)]
pub(super) use runs::FLOW_RUNS;
use runs::last_flow_run;
pub(super) use runs::{Run, RunKind, flow_runs, inline_runs, is_block_level};
use width::resolve_block_width;

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

/// Lay out `id`'s in-flow children per CSS 2.1 §10: its flow
/// (`flow::run`), each piece laid out in the document ([`LayoutSink`]) —
/// block runs block by block, inline runs in **anonymous block boxes**
/// (CSS 2.1 §9.2.1.1) that each establish their own IFC — then the
/// static positions of the out-of-flow children after the last in-flow
/// one.
///
/// Stores anonymous boxes on the parent's `TuiExt.anonymous_blocks`
/// — paint / hit-test / selection iterate this Vec alongside the
/// singular `inline_layout` field.
pub(super) fn layout_block_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    parent_computed: &ComputedStyle,
) -> BlockMeasurement {
    let mut statics = flow::Statics::default();
    let Some(prepared) = flow::prepare(dom, id, Some(&mut statics)) else {
        // Clear any stale anonymous boxes from a previous layout —
        // matches flex's `ext.inline_layout = None` reset.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.anonymous_blocks.clear();
        }
        let (scroll_x, scroll_y) = dom
            .node(id)
            .ext()
            .map_or((0, 0), |e| (e.scroll_x, e.scroll_y));
        for &n in &statics.trailing {
            super::positioning::record_static_position(
                dom,
                n,
                container.x - scroll_x,
                container.y - scroll_y,
            );
        }
        return BlockMeasurement::default();
    };
    let container = flow::inset(dom, &prepared, parent_computed, container);
    // This container's scroll offsets shift its cursor (mirrors
    // `flex::layout_flex_children`'s `container.y - scroll_main`;
    // `SCROLL-CROSS-AXIS-1`: horizontal scroll shifts every box left).
    let at = flow::FlowAt {
        container,
        scroll_x: super::gutter::scroll_offset(dom, id, Direction::Row),
        scroll_y: super::gutter::scroll_offset(dom, id, Direction::Column),
    };
    let mut sink = LayoutSink {
        dom,
        id,
        anon_blocks: Vec::new(),
        statics,
    };
    let end = flow::run(&mut sink, id, parent_computed, &prepared, at);
    let LayoutSink {
        dom,
        anon_blocks,
        statics,
        ..
    } = sink;

    // Positioned children after the last in-flow child: continue the
    // last inline run, or sit below the last block and the margin that
    // collapses with their zero-margin hypothetical box (CSS 2.1
    // §8.3.1: as if the box had a bottom border) — including a bottom
    // margin that escaped through the parent and so never reached the
    // accumulator.
    let runs = &prepared.runs;
    let last_run_is_inline = matches!(last_flow_run(runs).map(|r| r.kind), Some(RunKind::Inline));
    if !statics.trailing.is_empty() {
        let mut trailing_margin = end.margin_acc;
        if end.suppress_last_bottom
            && !last_run_is_inline
            && let Some(last) = last_flow_run(runs).and_then(|r| r.children.last()?.node())
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
                container.width,
                &mut memo,
            ));
        }
        let below_last_block = end.y_cursor + i32::from(trailing_margin.resolved());
        let content_x = container.x - at.scroll_x;
        for &n in &statics.trailing {
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
    end.measurement
}

/// [`flow::FlowSink`] for layout: each piece laid out in the document,
/// against the float area of its formatting context (`float::with_area`),
/// the anonymous boxes kept for `id`, the static positions recorded.
struct LayoutSink<'a> {
    dom: &'a mut Dom<TuiExt>,
    id: NodeId,
    anon_blocks: Vec<AnonymousIfc>,
    statics: flow::Statics,
}

impl flow::FlowSink for LayoutSink<'_> {
    fn dom(&self) -> &Dom<TuiExt> {
        self.dom
    }

    fn anchor(&mut self, child: NodeId, x: i32, y: i32) {
        for &n in self.statics.before.get(&child).into_iter().flatten() {
            super::positioning::record_static_position(self.dom, n, x, y);
        }
    }

    fn float(&mut self, item: BoxItem, at: super::float::Placement) {
        // Laid out at once, so its exclusion has the height it gets.
        let placed = super::float::place_in_block_flow(self.dom, item, at);
        super::float::lay_out(self.dom, self.id, placed, at.cb_width);
    }

    fn block(&mut self, child: NodeId, mut place: BlockPlace<'_>) -> i32 {
        let (y_cursor, cb) = (place.y_cursor, place.containing_block_width);
        let mut placed = super::float::with_area(self.dom, |dom, area| {
            place::place_block_child(dom, area, child, &mut place)
        });
        store_margin_chain_memo(self.dom, &placed.memo);
        layout_node(self.dom, child, placed.rect, cb);
        // `layout_node` finalizes an `auto` height from the laid-out
        // content (CSS 2.1 §10.6.3), which the pre-layout one may miss:
        // the cursor advances by the height the child got — of its
        // intended place, not of a `position: relative` shift, which must
        // not move siblings.
        let mut height = laid_out_height(self.dom, child, placed.rect.height);
        // §9.5: a formatting context root beside floats, taller than
        // placed, is placed again at its height.
        if let Some(rect) = super::float::with_area(self.dom, |dom, area| {
            place::replace_beside_floats(dom, area, child, &placed, height)
        }) {
            layout_node(self.dom, child, rect, cb);
            height = laid_out_height(self.dom, child, rect.height);
            placed.rect = rect;
        }
        place::advance(&placed, placed.rect.y, height, y_cursor, place.margin_acc)
    }

    fn generated(
        &mut self,
        host: NodeId,
        slot: crate::ext::PseudoSlot,
        at: generated::GeneratedPlace<'_>,
    ) -> Option<i32> {
        let (anon, bottom) = generated::lay_out(self.dom, host, slot, at)?;
        self.anon_blocks.push(anon);
        Some(bottom)
    }

    fn inline_run(&mut self, run: &Run, pseudos: RunPseudos, place: inline_run::RunPlace) -> u16 {
        let anon =
            inline_run::lay_out(self.dom, self.id, run, pseudos, place, &self.statics.before);
        let height = anon.rect.height;
        self.anon_blocks.push(anon);
        height
    }

    fn table_run(&mut self, run: &Run, at: LayoutRect) -> u16 {
        crate::render::layout_pass::table::layout_anonymous(self.dom, self.id, &run.children, at)
    }
}

/// The border-box height `child` got from `layout_node` (`fallback`
/// when it has no layout).
fn laid_out_height(dom: &Dom<TuiExt>, child: NodeId, fallback: u16) -> u16 {
    dom.node(child).layout_rect().map_or(fallback, |r| r.height)
}
