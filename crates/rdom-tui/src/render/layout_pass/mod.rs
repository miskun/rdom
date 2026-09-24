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
//! 1. **Main-axis sizing** (see [`flex`]). For `Row`, main = width;
//!    for `Column`, main = height. Children contribute:
//!    - `Fixed(n)` → `n` main-axis cells
//!    - `Auto` → intrinsic size ([`intrinsic`])
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
//! establishes an inline formatting context ([`ifc`]). Its children
//! don't participate in flex — they get zero-sized layout rects and
//! their paint is fragment-driven via `TuiExt.inline_layout`.
//!
//! ## Module layout
//!
//! - `mod.rs` — public `LayoutExt` trait + `layout_node` dispatch +
//!   shared helpers (element_children_of, parent_scroll) +
//!   fragment handling.
//! - [`flex`] — flex distribution: `layout_children`,
//!   `layout_flex_children`, `resolve_cross_size`.
//! - [`intrinsic`] — `Size::Auto` resolution via content
//!   measurement. Text / element / IFC paths.
//! - [`ifc`] — IFC detection.
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

mod block;
#[cfg(test)]
mod block_tests;
mod border_collapse;
mod flex;
mod ifc;
pub(crate) mod intrinsic;
mod positioned_pseudos;
mod positioning;
mod sticky;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Overflow, compute_content_area_collapsed};
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::style::ComputedStyle;

use flex::{layout_children, layout_flex_children};

pub(crate) use ifc::is_ifc_block;

/// Extension trait on `Dom<TuiExt>` adding `layout_dom(viewport)`.
pub trait LayoutExt {
    /// Run the layout pass against `viewport`. Writes `TuiExt.layout`
    /// and `TuiExt.content_layout` for every element. Safe to call
    /// repeatedly — each call fully re-lays out.
    fn layout_dom(&mut self, viewport: Rect);
}

impl LayoutExt for Dom<TuiExt> {
    fn layout_dom(&mut self, viewport: Rect) {
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
        // Pass 3 — place positioned `::before` / `::after` pseudo-
        // elements. Runs AFTER pass 2 so absolute pseudos whose hosts
        // are themselves absolute can read the host's placed rect.
        positioned_pseudos::place_positioned_pseudos(self, root_rect);
    }
}

// ─── Per-node layout ────────────────────────────────────────────────

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
    let parent_rect = dom
        .node(id)
        .parent_node()
        .and_then(|p| {
            use crate::node::TuiNodeExt;
            p.tui_ext().map(|e| e.content_layout)
        })
        .unwrap_or(outer_rect);
    let outer_rect = positioning::apply_relative_shift(&computed, outer_rect, parent_rect);

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
    }

    // Lay out children inside `inner`. The returned measurement
    // captures the margin-collapse-aware content extent for block-
    // flow elements (CSS 2.1 §10.6.3 — used below to resolve
    // `height: Auto` on this element).
    let measurement = layout_children(dom, id, inner, &computed);

    // Collapse the geometry of any `display:none` child subtree. The in-flow
    // layout above filters those children out (they take no space), so without
    // this they keep the rect they were last laid out with while VISIBLE — and
    // a stale rect drives paint / hit-test for a box that should generate none
    // (LAYOUT-DISPLAY-NONE-STALE-RECT). Freshly-hidden nodes already read zero;
    // this only matters on the visible→none transition for persistent nodes.
    collapse_hidden_children(dom, id);

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
        let (overflow_y, overflow_x) = match dom.node(id).ext() {
            Some(ext) => (
                auto_no_stable_y && ext.scroll_content_height > inner.height as usize,
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
            let measurement = layout_children(dom, id, inner_v2, &computed);
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
        let _ = layout_children(dom, id, final_inner, &computed);
        record_scroll_content_size(dom, id, final_inner, &computed);
    }
}

/// Walk `id`'s direct element children (transparently descending
/// through nested Fragments, the same way `element_children_of`
/// does) and write the union of their layout extents back to
/// `id`'s `TuiExt.scroll_content_{width,height}` — with the
/// parent's `scroll_{x,y}` *added back in* so the recorded size
/// is the un-scrolled content extent.
fn record_scroll_content_size(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    inner: LayoutRect,
    computed: &ComputedStyle,
) {
    // Static early-exit: only scrollable containers care.
    let needs = matches!(
        computed.overflow_x,
        Overflow::Scroll | Overflow::Auto | Overflow::Hidden
    ) || matches!(
        computed.overflow_y,
        Overflow::Scroll | Overflow::Auto | Overflow::Hidden
    );
    if !needs {
        return;
    }

    // Parent's own scroll offset — children's layout rects had this
    // subtracted from their main-axis cursor (see flex.rs::
    // layout_flex_children). Add it back to compute the un-scrolled
    // content extent.
    let (scroll_x, scroll_y) = match dom.node(id).ext() {
        Some(ext) => (ext.scroll_x as i32, ext.scroll_y as i32),
        None => return,
    };

    // Content extent = max(child.bottom) - min(child.top) along each
    // axis, with the parent's `scroll_{x,y}` added back so the result
    // is the un-scrolled extent. The min/max framing (rather than
    // anchoring on `inner.{x,y}`) is what makes `collapse_parent_edge_insets`'s
    // top/left layout-time shifts cleanly ignored: those insets push
    // the first child away from `inner` but the children's collective
    // extent is what overflow actually depends on, and that extent
    // is `max - min` regardless of where the first child sits inside
    // the inner rect.
    let mut min_x: Option<i32> = None;
    let mut min_y: Option<i32> = None;
    let mut max_right: i32 = 0;
    let mut max_bottom: i32 = 0;
    let mut any = false;
    for child in element_children_of(dom, id) {
        // Skip out-of-flow children: `display:none` takes no space and
        // positioned children are placed in phase-2 against their own CB, not
        // the parent's content area — neither enlarges the scroll extent.
        if !is_in_flow(dom, child) {
            continue;
        }
        if let Some(ext) = dom.node(child).ext() {
            let rect = ext.layout;
            let top = rect.y + scroll_y;
            let left = rect.x + scroll_x;
            let bottom = top + rect.height as i32;
            let right = left + rect.width as i32;
            min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
            min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
            max_right = max_right.max(right);
            max_bottom = max_bottom.max(bottom);
            any = true;
        }
    }

    // Text content: a pure-text leaf or IFC block packs its lines from
    // the top of `inner` (stored unscrolled), so its extent is the line
    // count by the widest line. Anonymous block boxes (mixed content)
    // carry scrolled rects like element children do. Without these a
    // `<textarea>` with six lines reported zero content and could never
    // scroll (HARDENING-2026-09 R5).
    if let Some(ext) = dom.node(id).ext() {
        if let Some(il) = ext.inline_layout.as_ref() {
            let widest = il.lines.iter().map(|l| l.width as i32).max().unwrap_or(0);
            // An editing host whose text ends in a newline has one more
            // row than the packer emits: the empty line the caret sits
            // on after that newline (a browser `<textarea>` shows it; a
            // `<pre>` does not). The caret code models the same row
            // (`caret::phantom_line_and_column`), so the extent must
            // include it or the caret can never be scrolled into view.
            let trailing_caret_line = i32::from(trailing_newline_caret_row(dom, id));
            let top = inner.y;
            let left = inner.x;
            min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
            min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
            max_right = max_right.max(left + widest);
            max_bottom = max_bottom.max(top + il.height() as i32 + trailing_caret_line);
            any = true;
        }
        for anon in &ext.anonymous_blocks {
            let top = anon.rect.y + scroll_y;
            let left = anon.rect.x + scroll_x;
            min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
            min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
            max_right = max_right.max(left + anon.rect.width as i32);
            max_bottom = max_bottom.max(top + anon.rect.height as i32);
            any = true;
        }
    }

    let (content_w, content_h) = if any {
        (
            (max_right - min_x.unwrap_or(inner.x)).max(0),
            (max_bottom - min_y.unwrap_or(inner.y)).max(0),
        )
    } else {
        (0, 0)
    };

    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_content_width = content_w as usize;
        ext.scroll_content_height = content_h as usize;
    }
}

/// True when `id` is an editing host (`<textarea>`, text `<input>`,
/// `contenteditable`) whose last text child ends with `\n`: the caret
/// can then stand on an empty row after that newline, which the line
/// packer does not emit as a line box.
fn trailing_newline_caret_row(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::node::TuiNodeExt;
    if !dom.node(id).is_editable() {
        return false;
    }
    let last_text = dom
        .node(id)
        .child_nodes()
        .filter(|c| c.node_type() == rdom_core::NodeType::Text)
        .last();
    last_text.is_some_and(|t| t.node_value().is_some_and(|v| v.ends_with('\n')))
}

/// Clamp `id`'s scroll offset to `[0, scroll size − viewport size]`
/// on each axis (CSS keeps `scrollTop`/`scrollLeft` in range as
/// content changes). Only scroll containers can hold a non-zero
/// offset, so non-scrollable elements are a no-op. The viewport is
/// the element's final `content_layout` — the region children are
/// laid out and clipped into (after the two-pass scrollbar gutter
/// reflow), so this max matches what the runtime's wheel / scrollbar
/// / scroll-into-view path can actually reach. Returns whether an
/// offset changed (the caller then re-lays-out the children at the
/// corrected position).
fn clamp_scroll_offset(dom: &mut Dom<TuiExt>, id: NodeId, computed: &ComputedStyle) -> bool {
    let scrolls = !matches!(computed.overflow_x, Overflow::Visible)
        || !matches!(computed.overflow_y, Overflow::Visible);
    if !scrolls {
        return false;
    }
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    let vp = ext.content_layout;
    let max_x = ext.scroll_content_width.saturating_sub(vp.width as usize);
    let max_y = ext.scroll_content_height.saturating_sub(vp.height as usize);
    let new_x = ext.scroll_x.min(max_x);
    let new_y = ext.scroll_y.min(max_y);
    if new_x == ext.scroll_x && new_y == ext.scroll_y {
        return false;
    }
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_x = new_x;
        ext.scroll_y = new_y;
    }
    true
}

/// CSS 2.1 §10.6.3: resolve `height: Auto` on a block-flow element
/// from the measured content extent. `gutter_rows` is what the
/// scrollbar reservation took off the content area's height; it
/// belongs to the box, so the outer height counts it.
///
/// Gating:
/// - the element's own `flow == Block` (a flex container's height is
///   already final from its parent's distribution / declared size);
/// - the parent's `flow` is also `Block` — Auto height on a flex
///   *item* means "stretch to the cross axis" (CSS Flexbox §7.5), and
///   the parent's flex pass already wrote that height;
/// - no explicit `Fixed` / `Percent` / `Calc` height;
/// - not `absolute` / `fixed`: `compute_placed_rect` owns that height
///   (auto there means "derive from `top` / `bottom` against the
///   containing block").
fn resolve_auto_height(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    measurement: Option<block::BlockMeasurement>,
    gutter_rows: u16,
) {
    let parent_is_block_flow = dom
        .node(id)
        .parent_node()
        .and_then(|p| {
            use crate::node::TuiNodeExt;
            p.tui_ext()
                .and_then(|e| e.computed.as_ref().map(|c| c.flow))
        })
        .map(|f| matches!(f, crate::layout::Flow::Block))
        .unwrap_or(true);
    let is_out_of_flow_positioned = matches!(
        computed.position,
        crate::layout::Position::Absolute | crate::layout::Position::Fixed
    );
    if !matches!(computed.height, crate::layout::Size::Auto)
        || !matches!(computed.flow, crate::layout::Flow::Block)
        || !parent_is_block_flow
        || is_out_of_flow_positioned
    {
        return;
    }
    // An IFC block or pure-text leaf has no `BlockMeasurement`; its
    // content extent is its packed line count (at least the one row
    // an empty editing host keeps for the caret).
    let Some(measurement) = measurement.or_else(|| {
        dom.node(id)
            .ext()
            .and_then(|e| e.inline_layout.as_ref())
            .map(|il| block::BlockMeasurement {
                content_height: il.height().max(1),
            })
    }) else {
        return;
    };
    let content_h = crate::layout::clamp_size(
        measurement.content_height,
        match computed.min_height {
            Some(crate::layout::MinSize::Cells(n)) => Some(n),
            _ => None,
        },
        computed.max_height,
    );
    // Padding percent / calc resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.4) — the same basis
    // `compute_content_area_collapsed` used for this element's inset.
    let pad = computed.padding.top.resolve(containing_block_width)
        + computed.padding.bottom.resolve(containing_block_width);
    let border = computed.border.top.cells() + computed.border.bottom.cells();
    let outer_h = content_h
        .saturating_add(pad)
        .saturating_add(border)
        .saturating_add(gutter_rows);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout.height = outer_h;
        ext.content_layout.height = content_h;
    }
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
    // Fragment uses a Column-like default with no gap/padding —
    // treat it like an invisible Column container.
    let fallback = ComputedStyle::initial();
    layout_flex_children(dom, &children, container, &fallback);
}

// ─── Tree helpers ───────────────────────────────────────────────────

/// Direct *element* children of `id`, document order. Text/Comment
/// are skipped (they have no TuiExt and flow inline via intrinsic
/// measurement). Fragment children are unwrapped — their element
/// descendants are returned as if they were direct children of `id`.
pub(super) fn element_children_of(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    collect_element_children(dom, id, &mut out);
    out
}

/// True iff `id` participates in normal flow. Non-elements (text, comments,
/// fragments) always do; an element does when it's neither `display: none` nor
/// out-of-flow positioned (`absolute` / `fixed`). The single source of truth
/// for the "skip out-of-flow children" filter shared by block + flex layout and
/// the scroll-content walk (DRY-1).
/// Resolve a `gap` for `computed`'s children along `axis` (CSS Box
/// Alignment 3 §8): percentages resolve against the container's
/// content size on that axis, and against 0 when that size is
/// indefinite — which for rdom means an `auto`-height container's
/// block axis.
pub(super) fn resolve_gap(
    computed: &crate::style::ComputedStyle,
    container: LayoutRect,
    axis: Direction,
) -> u16 {
    let basis = match axis {
        Direction::Row => container.width,
        Direction::Column if computed.height == crate::layout::Size::Auto => 0,
        Direction::Column => container.height,
    };
    computed.gap.resolve(basis)
}

pub(crate) fn is_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return true; // text, comments, fragments
    }
    let Some(c) = node.ext().and_then(|e| e.computed.as_ref()) else {
        return true;
    };
    use crate::layout::{Display, Position};
    c.display != Display::None && !matches!(c.position, Position::Absolute | Position::Fixed)
}

/// Zero the layout geometry of every `display:none` child subtree of `id`.
/// In-flow layout filters `display:none` children out, so they'd otherwise
/// retain the rect from when they were last visible (LAYOUT-DISPLAY-NONE-STALE-
/// RECT). A `display:none` box generates no box, so its rect — and every
/// descendant's, since the subtree isn't laid out — must read zero.
fn collapse_hidden_children(dom: &mut Dom<TuiExt>, id: NodeId) {
    for child in element_children_of(dom, id) {
        let hidden = dom
            .node(child)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.display == crate::layout::Display::None)
            .unwrap_or(false);
        if hidden {
            collapse_subtree_geometry(dom, child);
        }
    }
}

/// Recursively reset `layout` / `content_layout` to the zero rect for `id` and
/// every element descendant. Used to collapse a `display:none` subtree.
fn collapse_subtree_geometry(dom: &mut Dom<TuiExt>, id: NodeId) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        if ext.layout == LayoutRect::default() && ext.content_layout == LayoutRect::default() {
            // Already collapsed — and so is everything below it (we always zero
            // top-down), so stop early. Keeps steady-state hidden subtrees O(1).
            return;
        }
        ext.layout = LayoutRect::default();
        ext.content_layout = LayoutRect::default();
    }
    for child in element_children_of(dom, id) {
        collapse_subtree_geometry(dom, child);
    }
}

fn collect_element_children(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element => out.push(child.id()),
            NodeType::Fragment => collect_element_children(dom, child.id(), out),
            NodeType::Text | NodeType::Comment => {}
        }
    }
}

/// Scroll offset for the parent container along `direction`. Reads
/// the *first* child's ext-parent to find the scroll config — since
/// all children share the same parent.
pub(super) fn parent_scroll(dom: &Dom<TuiExt>, children: &[NodeId], direction: Direction) -> i32 {
    let Some(&first) = children.first() else {
        return 0;
    };
    let Some(parent) = dom.node(first).parent_node() else {
        return 0;
    };
    let Some(ext) = parent.ext() else { return 0 };
    match direction {
        Direction::Row => ext.scroll_x as i32,
        Direction::Column => ext.scroll_y as i32,
    }
}

/// Shrink `inner` by a 1-cell scrollbar gutter per axis when CSS
/// `scrollbar-gutter` says to reserve it (or when `overflow:
/// scroll` requires a permanent gutter).
///
/// Reservation rules per axis:
/// - `Overflow::Scroll` → always reserve (scrollbar always shown).
/// - `Overflow::Auto` + `scrollbar-gutter: stable` → reserve
///   (matches CSS `scrollbar-gutter: stable` — prevents content
///   reflow when the scrollbar appears mid-frame).
/// - `Overflow::Auto` + `scrollbar-gutter: auto` (the CSS
///   default) → DO NOT reserve. The scrollbar paints over the
///   edge column/row only while it's visible; content gets the
///   cells when scrolling isn't active. Authors who want stable
///   layout opt in with `scrollbar-gutter: stable`.
/// - `Overflow::Hidden` / `Visible` → never reserve.
///
/// The reserved cells live at:
/// - **Vertical scrollbar** (if `overflow_y` reserves): the
///   rightmost column of `inner`, from top to bottom.
/// - **Horizontal scrollbar** (if `overflow_x` reserves): the
///   bottom row of `inner`, from left to right.
///
/// When both reserve, the bottom-right corner cell is unclaimed
/// by either strip — paint leaves it blank.
///
/// `force_y` / `force_x` override the cascade decision for `Auto`
/// axes — used by `layout_node`'s two-pass re-layout when overflow
/// was detected in pass 1. `Scroll` always reserves regardless; CSS
/// Overflow 3 §3 "classic" semantic for `Auto` ("consumes space when
/// present") needs the override because at the time of pass 1 the
/// substrate doesn't yet know if overflow will exist. Two-pass:
/// measure → if overflow on an Auto axis, force-reserve in pass 2.
pub(super) fn reserve_scrollbar_gutter_forced(
    inner: LayoutRect,
    computed: &ComputedStyle,
    force_y: bool,
    force_x: bool,
) -> LayoutRect {
    let (reserve_y, reserve_x) = gutter_axes(computed, force_y, force_x);
    LayoutRect::new(
        inner.x,
        inner.y,
        if reserve_y {
            inner.width.saturating_sub(1)
        } else {
            inner.width
        },
        if reserve_x {
            inner.height.saturating_sub(1)
        } else {
            inner.height
        },
    )
}

/// Which axes reserve a scrollbar gutter: `(vertical bar, horizontal
/// bar)`. `Scroll` always; `Auto` when forced (pass 2 saw overflow) or
/// under `scrollbar-gutter: stable`; never otherwise.
pub(super) fn gutter_axes(computed: &ComputedStyle, force_y: bool, force_x: bool) -> (bool, bool) {
    use crate::layout::ScrollbarGutter;
    let reserves = |o: Overflow, force: bool| match o {
        Overflow::Scroll => true,
        Overflow::Auto => force || matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable),
        Overflow::Hidden | Overflow::Visible => false,
    };
    (
        reserves(computed.overflow_y, force_y),
        reserves(computed.overflow_x, force_x),
    )
}

/// Pass-1 gutter reservation — Scroll always, Auto only if
/// `scrollbar-gutter: stable`. `Auto` without `stable` waits for
/// overflow detection then forces the gutter in pass 2 via
/// [`reserve_scrollbar_gutter_forced`].
pub(super) fn reserve_scrollbar_gutter(inner: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    reserve_scrollbar_gutter_forced(inner, computed, false, false)
}
