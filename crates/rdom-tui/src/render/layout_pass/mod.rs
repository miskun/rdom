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
        layout_node(self, root, root_rect);
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
pub(super) fn layout_node(dom: &mut Dom<TuiExt>, id: NodeId, outer_rect: LayoutRect) {
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
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);

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
    let inner = compute_content_area_collapsed(
        outer_rect,
        computed.padding.clone(),
        computed.border,
        computed.border_collapse,
    );

    // Further reduce `inner` by a 1-cell scrollbar gutter on each
    // axis with `Scroll` / `Auto` overflow. Matches CSS
    // `scrollbar-gutter: stable` — the cell is reserved even when
    // the `auto` case doesn't end up showing a thumb, so children
    // never reflow when a scrollbar appears/disappears. v1 uses a
    // fixed 1-cell scrollbar (no `scrollbar-width` property).
    let inner = reserve_scrollbar_gutter(inner, &computed);

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

    // CSS 2.1 §10.6.3 — Phase 6.1: resolve `height: Auto` on a
    // block-flow element against the measured content extent.
    //
    // Gating:
    // - element's own `flow == Block` (otherwise flex distribution
    //   inside this element governs its own children, but the
    //   element's height is already-final from above).
    // - parent's `flow` is also `Block` — Auto height on a flex
    //   *item* means "stretch to cross axis" (CSS Flexbox §7.5),
    //   not "intrinsic content," and the parent's flex pass has
    //   already written that height into our outer_rect. Touching
    //   it would clobber the stretch.
    // - element has an explicit `Fixed` / `Percent` / `Calc` height:
    //   already drives `inner.height`; skip the override.
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
    // Absolute / fixed elements get their height from
    // `compute_placed_rect` (positioning::place_positioned) — auto
    // height there means "derive from top/bottom against CB", NOT
    // "intrinsic content." Don't clobber that with the block
    // measurement.
    let is_out_of_flow_positioned = matches!(
        computed.position,
        crate::layout::Position::Absolute | crate::layout::Position::Fixed
    );
    if let Some(measurement) = measurement
        && matches!(computed.height, crate::layout::Size::Auto)
        && matches!(computed.flow, crate::layout::Flow::Block)
        && parent_is_block_flow
        && !is_out_of_flow_positioned
    {
        let content_h = crate::layout::clamp_size(
            measurement.content_height,
            match computed.min_height {
                Some(crate::layout::MinSize::Cells(n)) => Some(n),
                _ => None,
            },
            computed.max_height,
        );
        // Padding percent / calc resolves against the containing-block
        // width on ALL four sides (CSS 2.1 §8.4). Use the element's own
        // outer width here — same basis `compute_content_area_collapsed`
        // already uses for this element's inset.
        let pad_cb_w = outer_rect.width;
        let pad =
            computed.padding.top.resolve(pad_cb_w) + computed.padding.bottom.resolve(pad_cb_w);
        let border = computed.border.top as u16 + computed.border.bottom as u16;
        let outer_h = content_h.saturating_add(pad).saturating_add(border);
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.layout.height = outer_h;
            ext.content_layout.height = content_h;
        }
    }

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
            );
            let inner_v2 =
                reserve_scrollbar_gutter_forced(inner_full, &computed, overflow_y, overflow_x);
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.content_layout = inner_v2;
            }
            let _ = layout_children(dom, id, inner_v2, &computed);
            record_scroll_content_size(dom, id, inner_v2, &computed);
        }
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
        if let Some(ext) = dom.node(child).ext() {
            // Skip out-of-flow children (display:none has
            // layout=default zero; positioned children get placed
            // in phase-2 against their own CB, not the parent's
            // content area — they shouldn't enlarge the parent's
            // scroll content extent).
            let display = ext.computed.as_ref().map(|c| c.display);
            let position = ext.computed.as_ref().map(|c| c.position);
            if display == Some(crate::layout::Display::None) {
                continue;
            }
            if matches!(
                position,
                Some(crate::layout::Position::Absolute) | Some(crate::layout::Position::Fixed)
            ) {
                continue;
            }
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
    use crate::layout::ScrollbarGutter;
    let reserves = |o: Overflow, force: bool| match o {
        Overflow::Scroll => true,
        Overflow::Auto => force || matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable),
        Overflow::Hidden | Overflow::Visible => false,
    };
    let reserve_y = reserves(computed.overflow_y, force_y);
    let reserve_x = reserves(computed.overflow_x, force_x);
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

/// Pass-1 gutter reservation — Scroll always, Auto only if
/// `scrollbar-gutter: stable`. `Auto` without `stable` waits for
/// overflow detection then forces the gutter in pass 2 via
/// [`reserve_scrollbar_gutter_forced`].
pub(super) fn reserve_scrollbar_gutter(inner: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    reserve_scrollbar_gutter_forced(inner, computed, false, false)
}
