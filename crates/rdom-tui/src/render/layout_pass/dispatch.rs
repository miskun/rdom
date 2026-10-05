//! Which formatting context lays out an element's children —
//! [`layout_children`]: an inline formatting context, a pure-text leaf,
//! block flow, flex layout or grid layout.

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::LayoutRect;
use crate::style::ComputedStyle;

use super::ifc::is_ifc_block;
use super::{element_children_of, layout_node};

/// Lay out the children of `id` inside `container`: an inline
/// formatting context, a pure-text leaf, block flow, or — for a flex or
/// grid container — its items (CSS Flexbox §4, CSS Grid 2 §6.1), by
/// flex layout or grid layout.
///
/// Returns `Some(BlockMeasurement)` ONLY when this dispatch went
/// through the `Flow::Block` arm — that's the only path where
/// `layout_node` should override the element's `Auto` height with
/// the measured content extent. IFC / pure-text-leaf / flex / grid
/// paths return `None` because their own height already lands correctly
/// (IFC + pure-text via `inline_layout.height()`; flex and grid via the
/// parent's distribution of their measured content size).
pub(super) fn layout_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    computed: &ComputedStyle,
) -> Option<super::block::BlockMeasurement> {
    // A block container that establishes a block formatting context
    // (CSS 2.1 §9.4.1) holds its own floats: an exclusion area of its
    // own while its children are laid out (`float`).
    let owns_floats = computed.flow.is_block_flow()
        && (super::block::establishes_bfc(dom, id, computed) || !super::float::in_context(dom));
    if !owns_floats {
        return lay_out_children(dom, id, container, computed);
    }
    super::float::enter(dom);
    let measurement = lay_out_children(dom, id, container, computed);
    let area = super::float::leave(dom);
    // CSS 2.1 §10.6.7: a block formatting context root's automatic height
    // reaches the bottom margin edge of its lowest float.
    let Some(lowest) = area.lowest() else {
        return measurement;
    };
    let scroll_y = super::gutter::scroll_offset(dom, id, crate::layout::Direction::Column);
    let floats = (lowest - (container.y - scroll_y)).clamp(0, i32::from(u16::MAX)) as u16;
    let content = measurement.map(|m| m.content_height).or_else(|| {
        dom.node(id)
            .ext()
            .and_then(|e| e.inline_layout.as_ref())
            .map(|il| il.height())
    });
    match content {
        Some(h) if h >= floats => measurement,
        h => Some(super::block::BlockMeasurement {
            content_height: floats.max(h.unwrap_or(0)),
        }),
    }
}

/// [`layout_children`] in whatever float context is current.
fn lay_out_children(
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
        // Every arm that places a floated pseudo-element keeps it again.
        ext.floated_pseudos = None;
        // Only the grid arm records its lines.
        ext.grid_lines = None;
    }

    let kind = children_layout(dom, id, computed);
    // IFC block: inline element children don't participate in flex
    // layout — they're painted by the inline flow pass. Give each a
    // zero-sized layout rect (hit tests and debug tools shouldn't
    // crash on missing data; paint reads the parent's inline_layout
    // instead).
    if kind == ChildrenLayout::Inline {
        for child in element_children_of(dom, id) {
            if let Some(ext) = dom.node_mut(child).ext_mut() {
                ext.layout = LayoutRect::new(container.x, container.y, 0, 0);
                ext.content_layout = ext.layout;
                ext.layout_dirty = false;
                ext.margin_chain = None;
            }
        }
        // The lines sit in the *scrolled* content rect, as paint and
        // hit-test read them back.
        let lines_at = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(container);
        // Compute + store the inline layout at the block's final
        // content width, beside the floats of its formatting context.
        // Paint reads this back directly.
        let (mut inline_layout, floats) = pack_around_floats(dom, id, lines_at, container.width);
        super::generated_atoms::lay_out(dom, &mut inline_layout);
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
        for placed in floats {
            super::float::lay_out(dom, id, placed, container.width);
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
    // text</p>`): its rendered text still needs to wrap AND its caret
    // needs an inline-flow container to anchor to (`children_layout`).
    if kind == ChildrenLayout::TextLeaf {
        let lines_at = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(container);
        let (mut inline_layout, floats) = pack_around_floats(dom, id, lines_at, container.width);
        super::generated_atoms::lay_out(dom, &mut inline_layout);
        super::positioning::record_static_positions_in_ifc(dom, id, &inline_layout, lines_at);
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.inline_layout = Some(inline_layout);
        }
        for placed in floats {
            super::float::lay_out(dom, id, placed, container.width);
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
    // maps to Display::Block + Flow::Flex per CSS3 Display Module), and
    // into grid with `display: grid`.
    //
    // Note: this branch runs ONLY after the IFC + pure-text-leaf
    // carve-outs above. Both of those paths must stay above the
    // dispatch — they're not parameterized by Flow.
    let anonymous = match kind {
        ChildrenLayout::Block => {
            // Stale anon boxes from a prior flex layout: clear so
            // the new block layout starts fresh. Anon boxes will
            // be repopulated by `layout_block_children`. Only this
            // arm produces a `BlockMeasurement` that
            // `layout_node`'s `Auto`-height override consumes.
            return Some(super::block::layout_block_children(
                dom, id, container, computed,
            ));
        }
        ChildrenLayout::Flex => super::flex::layout_flex_container(dom, id, container, computed),
        ChildrenLayout::Grid => super::grid::layout_grid_children(dom, id, container, computed),
        ChildrenLayout::Inline | ChildrenLayout::TextLeaf => {
            unreachable!("laid out above")
        }
    };
    store_anonymous(dom, id, anonymous);
    // Flex and grid layout set each item's outer rect inside the
    // container; the container's own height was determined by its
    // parent's distribution / its declared size. Auto height on a
    // flex or grid container resolves via the parent's distribution +
    // `intrinsic_size`, not via a children-walk. Return `None` so
    // `layout_node` leaves our height alone.
    None
}

/// Which formatting context lays out an element's children — the one
/// decision layout ([`layout_children`]) and intrinsic measurement
/// (`intrinsic`, `block::measure`) both read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildrenLayout {
    /// An inline formatting context (`ifc::is_ifc_block`).
    Inline,
    /// A pure-text leaf: direct text (through box-less children) and no
    /// in-flow element child, packed as its own lines — `<textarea>`,
    /// `<input>`, `<p>only text</p>`. Not an IFC by `is_ifc_block`'s
    /// carve-out (paint routing for `::before` / `::after` chrome).
    TextLeaf,
    /// Block flow (CSS 2.1 §9.4.1), its inline runs in anonymous block
    /// boxes.
    Block,
    /// A flex container's items.
    Flex,
    /// A grid container's items.
    Grid,
}

/// Which formatting context lays out `id`'s children ([`ChildrenLayout`]).
pub(super) fn children_layout(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> ChildrenLayout {
    if is_ifc_block(dom, id) {
        return ChildrenLayout::Inline;
    }
    // A flex or grid container's text is its anonymous items' (CSS
    // Flexbox §4, CSS Grid 2 §6.1).
    match computed.flow {
        crate::layout::Flow::Flex => return ChildrenLayout::Flex,
        crate::layout::Flow::Grid => return ChildrenLayout::Grid,
        crate::layout::Flow::Block | crate::layout::Flow::FlowRoot => {}
    }
    // Text in a box-less child is this box's text (CSS Display 3 §2.5).
    // Empty text (an unsubmitted `<input>` / `<textarea>`) still
    // qualifies: the caret has to land somewhere, so the inline layout
    // is computed even when its lines list is empty.
    let has_text_child = crate::render::box_tree::holds_loose_text(dom, id, &|_| true);
    // Only *in-flow* element children disqualify the pure-text-leaf path:
    // out-of-flow children (`position: absolute|fixed`) don't participate in the
    // block/inline mix, so a "text + an absolutely-positioned child" element
    // (e.g. a chip with an absolute dropdown) is still a text leaf — it must use
    // its own `inline_layout` (so `::before`/`::after` + own text paint once via
    // Path 3, not duplicated by an anonymous block; see TREE-BFC-PSEUDO-1).
    let no_in_flow_element_children = element_children_of(dom, id)
        .iter()
        .all(|&c| !super::is_in_flow(dom, c));
    // A block-level `::before` / `::after` makes it a block container
    // with an anonymous box for its text.
    let block_pseudos = crate::render::inline::generated::block_pseudos(dom, id);
    let block_pseudo = block_pseudos.before || block_pseudos.after;
    if has_text_child && no_in_flow_element_children && !block_pseudo {
        ChildrenLayout::TextLeaf
    } else {
        ChildrenLayout::Block
    }
}

/// Pack `id`'s inline content at `width` cells, its lines starting at
/// `lines_at`, beside the floats of the formatting context it is in and
/// placing its own there (CSS 2.1 §9.5): the layout, and the floats with
/// the border boxes to lay them out at.
fn pack_around_floats(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    lines_at: LayoutRect,
    width: u16,
) -> (
    crate::render::inline::InlineLayout,
    Vec<super::float::PlacedFloat>,
) {
    let content = LayoutRect::new(lines_at.x, lines_at.y, width, lines_at.height);
    super::float::with_area(dom, |dom, area| {
        let mut ex = super::float::lines::InlineFloats::new(dom, area, content, content.y);
        let layout =
            crate::render::inline::compute_inline_layout_around(dom, id, width, Some(&mut ex));
        (layout, ex.into_placed())
    })
}

/// Keep a flex or grid container's anonymous items' boxes, in document
/// order (paint, hit-testing and the caret find a text's box by its
/// `child_range`).
fn store_anonymous(dom: &mut Dom<TuiExt>, id: NodeId, mut anonymous: Vec<AnonymousIfc>) {
    anonymous.sort_by_key(|a| a.child_range.0);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.anonymous_blocks = anonymous;
    }
}
