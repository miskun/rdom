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
//! - `mod.rs` — public `LayoutExt` trait + `layout_node` dispatch +
//!   shared helpers (element_children_of) +
//!   fragment handling.
//! - `dispatch` — `layout_children`: which formatting context lays out
//!   an element's children.
//! - `flex` — flex distribution: `layout_flex_container`,
//!   `layout_flex_children`, `resolve_cross_size`.
//! - `intrinsic` — `Size::Auto` resolution via content
//!   measurement. Text / element / IFC paths.
//! - `grid` — grid layout: `layout_grid_children`, the track sizing
//!   algorithm, a grid container's content size.
//! - `items` — the items a flex or grid container lays out (elements,
//!   pseudo-elements, anonymous items for runs of text).
//! - `ifc` — IFC detection.
//! - `tree` — element children, the in-flow predicate,
//!   `display: none` geometry reset.
//! - `auto_height` — `height: auto` from the measured content.
//! - `scroll_extent` — scrollable content extent, scroll clamp.
//! - `gutter` — scroll offsets and scrollbar gutters.
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
//! intrinsic measurement.

mod auto_height;
mod block;
#[cfg(test)]
mod block_tests;
mod border_collapse;
pub(crate) mod box_sizing;
mod dispatch;
mod distribution;
mod flex;
pub(crate) mod geometry;
mod grid;
pub(crate) mod gutter;
mod ifc;
pub(crate) mod intrinsic;
mod items;
mod margin_trim;
mod positioned_pseudos;
mod positioning;
mod scroll_extent;
mod shares;
mod sticky;
mod tree;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Overflow, compute_content_area_collapsed};
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::style::ComputedStyle;

use dispatch::layout_children;
use flex::layout_flex_children;

use auto_height::resolve_auto_height;
pub(crate) use grid::GridLines;
pub(super) use gutter::{gutter_axes, reserve_scrollbar_gutter, reserve_scrollbar_gutter_forced};
pub(crate) use ifc::is_ifc_block;
use scroll_extent::{clamp_scroll_offset, record_scroll_content_size};
pub(crate) use scroll_extent::{
    origin_at_end, scroll_x_bounds, scroll_x_from_area_start, scroll_y_bounds,
    scroll_y_from_area_start,
};
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
        // Intrinsic sizes are memoized for this pass only.
        intrinsic::begin_pass(self);
        crate::style::cascade::set_document_viewport(
            self,
            rdom_style::calc::Viewport::new(viewport.width, viewport.height),
        );
        let root = self.root();
        let root_rect = LayoutRect::new(
            viewport.x as i32,
            viewport.y as i32,
            viewport.width,
            viewport.height,
        );
        // Pass 1 — flex / inline flow. Skips position: absolute /
        // fixed children at every container (see flex.rs filter).
        layout_node(self, root, root_rect, root_rect.width);
        // Pass 2 — place absolute / fixed elements against their
        // containing blocks.
        positioning::place_positioned(self, root_rect);
        // Pass 2.5 — place position: sticky elements. They stayed
        // in flow during pass 1; this pass adjusts their rect based
        // on the nearest scrollable ancestor's scroll position.
        sticky::place_sticky(self);
        #[cfg(debug_assertions)]
        block::debug_assert_no_margin_chain_memo(self, root);
        // Pass 3 — place positioned `::before` / `::after` pseudo-
        // elements. Runs AFTER pass 2 so absolute pseudos whose hosts
        // are themselves absolute can read the host's placed rect.
        positioned_pseudos::place_positioned_pseudos(self, root_rect);
        intrinsic::end_pass(self);
    }
}

// ─── Per-node layout ────────────────────────────────────────────────

#[cfg(test)]
thread_local! {
    /// Elements [`layout_node`] laid out (cost tests).
    pub(super) static LAYOUTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Lay out `id` as occupying `outer_rect`, then recurse into
/// children using this element's `content_layout` as their container.
/// `containing_block_width` is the width percent padding and margins
/// resolve against (CSS 2.1 §8.3 / §8.4): the parent's content width
/// for in-flow boxes, the containing block's for positioned ones.
pub(super) fn layout_node(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    outer_rect: LayoutRect,
    containing_block_width: u16,
) {
    // Skip non-elements — they have no TuiExt. Fragment children
    // are visited when the parent iterates its children (text /
    // comment get pulled into intrinsic measurements).
    if dom.node(id).node_type() != NodeType::Element {
        // Fragments *do* propagate layout to their element children
        // transparently. For a Fragment root (the default rdom-core
        // root), we still want children laid out within outer_rect.
        if dom.node(id).node_type() == NodeType::Fragment {
            layout_fragment_children(dom, id, outer_rect);
        }
        return;
    }

    #[cfg(test)]
    LAYOUTS.with(|c| c.set(c.get() + 1));
    let computed = dom
        .node(id)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

    // Apply the `position: relative` shift before everything else
    // so children flow inside the *shifted* content area. Siblings
    // already had their rects written by the parent's layout_children
    // loop (which advances its cursor by the in-flow `size`, not the
    // shifted rect), so they don't see the shift — matching CSS.
    // Pass the parent's content_layout for percentage basis on
    // `top`/`bottom` (parent height) and `left`/`right` (parent width).
    // The box parent's: a `display: contents` parent has no box.
    let parent_rect = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| {
            use crate::node::TuiNodeExt;
            dom.node(p).tui_ext().map(|e| e.content_layout)
        })
        .unwrap_or(outer_rect);
    // CSS 2.1 §9.3.2: a percentage `top` / `bottom` is `auto` when the
    // containing block's height is not specified explicitly — so an
    // `auto` parent height, still an estimate at this point, is never
    // a basis.
    let parent_height_definite = block::nearest_block_ancestor_height_is_definite(dom, id);
    let outer_rect = positioning::apply_relative_shift(
        &computed,
        outer_rect,
        parent_rect,
        parent_height_definite,
    );

    // Inset by this element's own padding + border. Under
    // `border-collapse: collapse`, an element with a border has its
    // content area expanded to include the border ring (decision 2,
    // M5.5b) — children's outer edges then coincide with the parent's
    // border cells.
    let content_area = compute_content_area_collapsed(
        outer_rect,
        computed.padding.clone(),
        computed.border,
        computed.border_collapse,
        containing_block_width,
    );

    // Further reduce `inner` by a 1-cell scrollbar gutter on each
    // axis with `Scroll` / `Auto` overflow. Matches CSS
    // `scrollbar-gutter: stable` — the cell is reserved even when
    // the `auto` case doesn't end up showing a thumb, so children
    // never reflow when a scrollbar appears/disappears. v1 uses a
    // fixed 1-cell scrollbar (no `scrollbar-width` property).
    let inner = reserve_scrollbar_gutter(content_area, &computed);

    // Write our rects.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout = outer_rect;
        ext.content_layout = inner;
        ext.layout_dirty = false;
        ext.margin_chain = None;
    }

    // Lay out children inside `inner`. The returned measurement
    // captures the margin-collapse-aware content extent for block-
    // flow elements (CSS 2.1 §10.6.3 — used below to resolve
    // `height: Auto` on this element).
    let measurement = layout_children_aligned(dom, id, inner, &computed, containing_block_width);

    // Collapse the geometry of any `display:none` child subtree. The in-flow
    // layout above filters those children out (they take no space), so without
    // this they keep the rect they were last laid out with while VISIBLE — and
    // a stale rect drives paint / hit-test for a box that should generate none
    // (LAYOUT-DISPLAY-NONE-STALE-RECT). Freshly-hidden nodes already read zero;
    // this only matters on the visible→none transition for persistent nodes.
    let content_box = dom.node(id).ext().map_or(inner, |e| e.content_layout);
    collapse_hidden_children(dom, id, content_box);

    // CSS 2.1 §10.6.3 — resolve `height: Auto` on a block-flow element
    // against the measured content extent, plus the gutter rows the
    // scrollbar reservation took out of the content area.
    resolve_auto_height(
        dom,
        id,
        &computed,
        containing_block_width,
        measurement,
        content_area.height.saturating_sub(inner.height),
    );

    // Record the scrollable content extent (cells that children
    // occupied, in the parent's content-area coord space, scroll
    // offset *added back*). Scrollbar paint and the runtime's
    // wheel-scroll clamp read these — without them the scrollbar
    // can't tell viewport from content size, so the thumb fills
    // the whole track regardless of overflow state.
    record_scroll_content_size(dom, id, inner, &computed);

    // Two-pass classic scrollbar (CSS Overflow 3 §3): for `Auto`
    // axes without `scrollbar-gutter: stable`, we couldn't decide
    // at pass-1 time whether the scrollbar would be visible. Now
    // that `scroll_content_*` is known, force-reserve the gutter
    // on any Auto axis that actually overflowed, then redo the
    // children's layout one cell narrower / shorter. TUI cells
    // can't be overlay-composited — the spec's "classic platform
    // = scrollbars consume space when present" path is the only
    // one available to us, and pass 2 is how we honor it.
    //
    // Convergent in two passes: a narrower viewport can only
    // increase overflow, never decrease it, so the second pass's
    // gutter decision sticks.
    use crate::layout::ScrollbarGutter;
    let auto_no_stable_y = matches!(computed.overflow_y, Overflow::Auto)
        && !matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable);
    let auto_no_stable_x = matches!(computed.overflow_x, Overflow::Auto)
        && !matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable);
    if auto_no_stable_y || auto_no_stable_x {
        // Compare against the FINAL content height: an `auto` height
        // was just resolved from the content (CSS 2.1 §10.6.3 — such a
        // box cannot overflow its block axis unless `max-height`
        // clamps it), while the pass-1 `inner` still carries the
        // pre-layout estimate.
        let (overflow_y, overflow_x) = match dom.node(id).ext() {
            Some(ext) => (
                auto_no_stable_y && ext.scroll_content_height > ext.content_layout.height as usize,
                auto_no_stable_x && ext.scroll_content_width > inner.width as usize,
            ),
            None => (false, false),
        };
        if overflow_y || overflow_x {
            // Recompute inner from scratch (pass-1 inner already had
            // Scroll / Stable gutters applied; we add the Auto
            // gutter on top via the force flags).
            let inner_full = compute_content_area_collapsed(
                outer_rect,
                computed.padding.clone(),
                computed.border,
                computed.border_collapse,
                containing_block_width,
            );
            let inner_v2 =
                reserve_scrollbar_gutter_forced(inner_full, &computed, overflow_y, overflow_x);
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.content_layout = inner_v2;
            }
            // Pass 2 is a full re-layout: the content may wrap
            // differently in the narrower area and the forced gutter
            // row is part of this box, so the `auto` height resolves
            // again from the new measurement.
            let measurement =
                layout_children_aligned(dom, id, inner_v2, &computed, containing_block_width);
            resolve_auto_height(
                dom,
                id,
                &computed,
                containing_block_width,
                measurement,
                inner_full.height.saturating_sub(inner_v2.height),
            );
            record_scroll_content_size(dom, id, inner_v2, &computed);
        }
    }

    // Clamp a stale scroll offset to the content. CSS keeps
    // `scrollTop`/`scrollLeft` within `[0, scroll size − client size]`
    // at all times — so when a scroll container's content shrinks
    // (its subtree is replaced with shorter content, or children are
    // removed), a previously-valid offset that now exceeds the max
    // must snap back (to 0 when the content again fits). Without this
    // the container stays scrolled past its content: blank at the
    // bottom, top clipped, and — when the new content fits — no
    // scrollbar to reveal it. Runs LAST so it sees the final
    // `content_layout` (after the two-pass gutter reflow), and uses
    // that as the scroll viewport — the same region children are laid
    // out and clipped into, so the max matches the runtime's
    // wheel/scrollbar/scroll-into-view clamp to the cell. The recorded
    // `scroll_content_*` is offset-independent, so the max is stable;
    // if an offset changed, re-lay-out the children at the corrected
    // position. Cheap: the re-layout only runs when an offset was
    // actually stale.
    if clamp_scroll_offset(dom, id, &computed) {
        let final_inner = dom
            .node(id)
            .ext()
            .map(|e| e.content_layout)
            .unwrap_or(inner);
        let _ = layout_children_aligned(dom, id, final_inner, &computed, containing_block_width);
        record_scroll_content_size(dom, id, final_inner, &computed);
    }
    // The offsets the children were just placed with.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        crate::runtime::scrollbar::state::note_laid_out(ext);
    }
}

/// [`layout_children`], with a block container's content shifted on its
/// block axis by `align-content` (CSS Box Alignment 3 §5.1): laid out
/// once, measured, and moved to its offset (`tree::shift_content` —
/// layout is translation-invariant, so laying it out again at the offset
/// would only repeat the work, once per aligned ancestor).
fn layout_children_aligned(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    inner: LayoutRect,
    computed: &ComputedStyle,
    containing_block_width: u16,
) -> Option<block::BlockMeasurement> {
    let measurement = layout_children(dom, id, inner, computed);
    // The content's height: the measured block-level content, else the
    // lines of an inline formatting context or a text leaf.
    let content_height = match &measurement {
        Some(m) => Some(m.content_height),
        None => dom
            .node(id)
            .ext()
            .and_then(|e| e.inline_layout.as_ref())
            .map(|il| il.height()),
    };
    let lead = content_height.map_or(0, |h| {
        block::align_content_lead(dom, id, computed, inner, h, containing_block_width)
    });
    // Lines cannot start above the content box (`tree::shift_lines`):
    // overflowing inline content stays at the top (DIVERGENCES §4).
    let lead = if measurement.is_none() {
        lead.max(0)
    } else {
        lead
    };
    if lead != 0 {
        if measurement.is_none() {
            tree::shift_lines(dom, id, lead);
        }
        tree::shift_content(dom, id, lead);
    }
    measurement
}

/// Fragment case: children inherit our container rect directly
/// (no padding, no border, no layout-rect write for the fragment).
fn layout_fragment_children(dom: &mut Dom<TuiExt>, id: NodeId, container: LayoutRect) {
    // Same filter as `flex::layout_children`: out-of-flow children
    // (display:none, position:absolute|fixed) don't participate in
    // distribution. Positioned children get placed in phase-2
    // against their containing block (= the viewport, since a
    // Fragment is not a positioned containing block).
    let children: Vec<NodeId> = element_children_of(dom, id)
        .into_iter()
        .filter(|&c| is_in_flow(dom, c))
        .collect();
    for n in positioning::out_of_flow_positioned_children(dom, id) {
        positioning::record_static_position(dom, n, container.x, container.y);
    }
    // The fragment root lays its children out in rdom's viewport
    // column: an invisible column flex container with no gap or
    // padding (its children stretch to the viewport's width, as a
    // browser's `<body>` blocks do).
    let mut viewport_column = ComputedStyle::initial();
    viewport_column.flow = crate::layout::Flow::Flex;
    viewport_column.direction = Direction::Column;
    // Element items only: the root's text is not laid out.
    let anonymous = layout_flex_children(
        dom,
        &items::elements(&children),
        container,
        &viewport_column,
    );
    debug_assert!(
        anonymous.is_empty(),
        "element items make no anonymous boxes"
    );
    collapse_hidden_children(dom, id, container);
}

// ─── Tree helpers ───────────────────────────────────────────────────

/// The axis `computed`'s in-flow children are laid out along: a flex
/// container's main axis (`flex-direction`, CSS Flexbox §5.1), every
/// other box's block axis — vertical, as every box lays out
/// `horizontal-tb` (DIVERGENCES §1). `flex-direction` applies to flex
/// containers only, so it never turns a block container sideways.
pub(super) fn flow_axis(computed: &ComputedStyle) -> Direction {
    match computed.flow {
        crate::layout::Flow::Flex => computed.direction,
        // A grid container's items lay out on both axes; its block axis
        // stands for it (it takes no `border-collapse` insets, and its
        // content size is the grid's, `grid::content_size`).
        crate::layout::Flow::Block | crate::layout::Flow::FlowRoot | crate::layout::Flow::Grid => {
            Direction::Column
        }
    }
}

/// Resolve the gap between `computed`'s children laid out along `axis`
/// (CSS Box Alignment 3 §8.1): `column-gap` between items placed
/// horizontally, `row-gap` between items stacked vertically; `normal`
/// is 0. Percentages resolve against the container's content size on
/// that axis, and against 0 when that size is indefinite — which for
/// rdom means an `auto`-height container's block axis.
pub(super) fn resolve_gap(
    computed: &crate::style::ComputedStyle,
    container: LayoutRect,
    axis: Direction,
) -> u16 {
    let basis = match axis {
        Direction::Row => container.width,
        // An `auto` or keyword height (its content height, CSS Sizing 3
        // §3.1) is indefinite.
        Direction::Column
            if matches!(
                computed.height,
                crate::layout::Size::Auto | crate::layout::Size::Intrinsic(_)
            ) =>
        {
            0
        }
        Direction::Column => container.height,
    };
    gap_along(computed, axis).resolve(basis)
}

/// The gap property between children laid out along `axis`:
/// `column-gap` along the horizontal axis, `row-gap` along the vertical.
pub(super) fn gap_along(
    computed: &crate::style::ComputedStyle,
    axis: Direction,
) -> &crate::layout::GapValue {
    match axis {
        Direction::Row => &computed.column_gap,
        Direction::Column => &computed.row_gap,
    }
}
