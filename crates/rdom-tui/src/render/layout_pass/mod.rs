//! The layout pass.
//!
//! Walks `Dom<TuiExt>` in document order. For every element, reads
//! `ComputedStyle` (`direction`, `padding`, `border`, `gap`, `width`,
//! `height`, `min_*`, `max_*`, `overflow`) and writes the element's
//! position/size into `TuiExt.layout` and `TuiExt.content_layout`.
//!
//! ## Algorithm (flexbox subset)
//!
//! Given a container's `content_layout` (inner rect after padding +
//! border on the container itself) and its children:
//!
//! 1. **Main-axis sizing** (see `flex`). For `Row`, main = width;
//!    for `Column`, main = height. Children contribute:
//!    - `Fixed(n)` → `n` main-axis cells
//!    - `Auto` → intrinsic size (`intrinsic`)
//!    - `Flex(w)` → share of the remaining space proportional to `w`
//! 2. **Cross-axis sizing**: stretch to fill unless `Fixed(n)`.
//! 3. **Min/max clamping** per CSS rules.
//! 4. **Position children** along main axis with `gap` cells between.
//!    Apply parent's `scroll_{x,y}` as a negative offset.
//! 5. **Recurse** into each child's own layout using its
//!    `content_layout` as the container.
//!
//! ## Positioning
//!
//! `position: relative | absolute | fixed` adds a phase-2 placement
//! step. `positioning::containing_block` resolves the rect each
//! positioned element places against.
//!
//! ## IFC blocks
//!
//! A block whose element children are all `display: inline`
//! establishes an inline formatting context (`ifc`). Its children
//! don't participate in flex — they get zero-sized layout rects and
//! their paint is fragment-driven via `TuiExt.inline_layout`.
//!
//! ## Module layout
//!
//! - `mod.rs` — public `LayoutExt` trait, the whole-tree pass
//!   (`layout_once`) and the shared re-exports.
//! - `node` — one element's layout (`layout_node`).
//! - `icb` — the initial containing block (CSS 2.1 §10.1): the document
//!   root's children in block flow, or an element root as a block.
//! - `dispatch` — `layout_children`: which formatting context lays out
//!   an element's children.
//! - `flex` — flex distribution: `layout_flex_container`,
//!   `layout_flex_children`, `resolve_cross_size`.
//! - `intrinsic` — `Size::Auto` resolution via content
//!   measurement. Text / element / IFC paths.
//! - `grid` — grid layout: `layout_grid_children`, the track sizing
//!   algorithm, a grid container's content size.
//! - `items` — the items a flex or grid container lays out (elements,
//!   pseudo-elements, anonymous items for runs of text) and their
//!   baseline geometry.
//! - `distribution` — content distribution (`justify-content` /
//!   `align-content`), shared by flex and grid.
//! - `flow` — the axis children lay out along, and the gap between them.
//! - `ifc` — IFC detection.
//! - `tree` — element children, the in-flow predicate,
//!   `display: none` geometry reset.
//! - `auto_height` — `height: auto` from the measured content.
//! - `scroll_extent` — scrollable content extent, scroll clamp.
//! - `positioned_overflow` — absolutely positioned boxes in their
//!   scroll container's extent, measured after placement.
//! - `gutter` — scroll offsets and scrollbar gutters.
//! - `float` — floats: the exclusion area of a block formatting
//!   context, float placement, clearance, the bands line boxes use.
//! - `fragment` — block fragmentation (CSS Fragmentation 3): a laid-out
//!   flow's breaks, cut into fragmentainers and moved into them.
//! - `multicol` — multi-column containers (CSS Multi-column 1): their
//!   column boxes, the fragmentainers of their content.
//!
//! ## Scroll
//!
//! Applied at the container level: children of a scrolled parent
//! start at `content_layout.{x|y} - parent.scroll_{x|y}`. Negative
//! signed coords mean "scrolled off screen"; paint clips at positive
//! coords.
//!
//! ## Non-elements
//!
//! Text / Comment / Fragment nodes have no `TuiExt`. During layout
//! we skip them structurally (they don't occupy layout slots on
//! their own). Text content is consumed via the parent element's
//! intrinsic measurement; the root fragment's box is the initial
//! containing block (`icb`).

mod auto_height;
pub(crate) mod baselines;
mod block;
#[cfg(test)]
mod block_tests;
mod border_collapse;
pub(crate) mod box_sizing;
mod calc_size;
mod clip_edge;
pub(crate) mod container_pass;
pub(crate) mod containment;
mod dispatch;
mod distribution;
mod flex;
pub(crate) mod float;
mod flow;
pub(crate) mod fragment;
pub(crate) mod generated_atoms;
pub(crate) mod geometry;
mod grid;
pub(crate) mod gutter;
mod icb;
mod ifc;
pub(crate) mod intrinsic;
mod items;
pub(crate) mod line_clamp;
mod margin_trim;
pub(crate) mod multicol;
mod node;
mod positioned_overflow;
#[cfg(test)]
pub(crate) use positioned_overflow::MAX_ROUNDS;
mod picker;
mod positioning;
mod scroll_extent;
pub(crate) mod scroll_update;
pub(crate) mod scrollport;
mod shares;
mod sticky;
mod table;
mod tree;

#[cfg(test)]
mod idle_cost_tests;
#[cfg(test)]
mod tests;

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::Rect;

pub(super) use flow::{flow_axis, gap_along, resolve_gap};
#[cfg(test)]
pub(crate) use node::LAYOUTS;
pub(super) use node::layout_node;

pub(crate) use clip_edge::ClipEdges;
pub(crate) use grid::GridLines;
pub(super) use gutter::{gutters, reserve_scrollbar_gutter, reserve_scrollbar_gutter_forced};
pub(crate) use ifc::is_ifc_block;
pub(crate) use positioning::position_hidden;
pub(crate) use scroll_extent::origin_at_end;
pub(crate) use scrollport::{
    offset_from_area_start, overflows, range_of, scroll_bounds, scrollport, scrollport_of,
};
pub(crate) use table::{hides_empty_cell, is_column_box};
use tree::collapse_hidden_children;
pub(super) use tree::element_children_of;
pub(crate) use tree::is_in_flow;

/// Extension trait on `Dom<TuiExt>` adding `layout_dom(viewport)`.
pub trait LayoutExt: crate::sealed::Sealed {
    /// Run the layout pass against `viewport`. Writes `TuiExt.layout`
    /// and `TuiExt.content_layout` for every element. Safe to call
    /// repeatedly — each call fully re-lays out.
    ///
    /// Records `viewport`'s size as the document's viewport
    /// ([`CascadeExt::set_viewport`](crate::CascadeExt::set_viewport)),
    /// so the next cascade resolves `vw` / `vh` against it. Lay-out does
    /// not re-cascade: after a size change, cascade the whole tree again.
    fn layout_dom(&mut self, viewport: Rect);
}

impl LayoutExt for Dom<TuiExt> {
    fn layout_dom(&mut self, viewport: Rect) {
        // A query container's subtree is re-cascaded once its size is
        // known (`container_pass`); a `calc-size()`d box takes a second
        // pass (`calc_size`).
        container_pass::lay_out(self, viewport, |dom, viewport| {
            calc_size::lay_out(dom, viewport, layout_once);
        });
    }
}

#[cfg(test)]
thread_local! {
    /// Runs of phases 1–2 `layout_dom` made (cost tests).
    pub(crate) static ROUNDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// One whole-tree layout pass.
fn layout_once(dom: &mut Dom<TuiExt>, viewport: Rect) {
    // Intrinsic sizes are memoized for this pass only, and its clamp
    // points kept until the next.
    intrinsic::begin_pass(dom);
    line_clamp::begin_pass(dom);
    crate::style::cascade::set_document_viewport(
        dom,
        rdom_style::calc::Viewport::new(viewport.width, viewport.height),
    );
    let root = dom.root();
    let root_rect = LayoutRect::new(
        viewport.x as i32,
        viewport.y as i32,
        viewport.width,
        viewport.height,
    );
    // What this layout does after placing the positioned boxes, kept for a
    // scroll update to take back (`scroll_update`).
    scroll_update::begin(dom, root_rect);
    // Passes 1–2 run again while the absolutely positioned boxes'
    // reach into their scroll containers changes
    // (`positioned_overflow`): pass 1 records scroll extents with
    // the reach the last settle measured. Run 2 sees a new reach;
    // run 3 the reach of boxes whose containing block run 2's
    // scrollbar narrowed. A reach still moving after that toggles a
    // scrollbar back and forth and is kept for the next layout.
    for round in 0..positioned_overflow::MAX_ROUNDS {
        if round > 0 {
            intrinsic::end_pass(dom);
            intrinsic::begin_pass(dom);
        }
        #[cfg(test)]
        ROUNDS.with(|c| c.set(c.get() + 1));
        // Pass 1 — in-flow layout, from the initial containing block
        // down. Skips position: absolute / fixed children at every
        // container (see flex.rs filter).
        icb::lay_out(dom, root, root_rect);
        // Pass 2 — place absolute / fixed elements against their
        // containing blocks.
        let placed = positioning::place_positioned(dom, root_rect);
        if !positioned_overflow::settle(dom, &placed) {
            break;
        }
    }
    // Pass 2.5 — place position: sticky elements. They stayed
    // in flow during pass 1; this pass adjusts their rect based
    // on the nearest scrollable ancestor's scroll position.
    sticky::place_sticky(dom);
    // Pass 2.6 — an open `<select>` picker flips above its field where
    // the screen ends.
    picker::place_pickers(dom, root_rect);
    #[cfg(debug_assertions)]
    block::debug_assert_no_margin_chain_memo(dom, root);
    // Pass 3 — move relatively positioned and sticky `::before` /
    // `::after` from their in-flow places (phase 2 placed the
    // absolute and fixed ones with the elements).
    positioning::offset_in_flow_pseudos(dom);
    intrinsic::end_pass(dom);
    // The highlight layers the paints after this layout read, indexed
    // once (`highlight_index`).
    crate::render::highlight_index::prepare(dom);
}

// ─── Tree helpers ───────────────────────────────────────────────────
