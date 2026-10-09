//! Positioning — containing-block resolution + phase-2 placement
//! for `position: absolute | fixed` elements (M2).
//!
//! Containing-block resolution:
//!
//! - `position: fixed` → always the initial containing block (the
//!   root viewport).
//! - `position: absolute` → the nearest ancestor whose
//!   `position` is not `static`, or the viewport if none
//!   (`containing`, shared with positioned pseudo-elements).
//! - `position: relative` / `static` → returns the parent's
//!   layout rect; used for the §5 static-position fallback.
//!
//! Phase-2 placement walks the tree in document order and, for
//! every element with `position: absolute | fixed`, resolves its
//! containing block, computes the placed rect from
//! `top/right/bottom/left` + `width/height`, writes it into
//! `TuiExt.layout`, and re-runs `layout_node` on the subtree so
//! the element's own children flow inside the placed rect.
//!
//! An axis whose two insets are both `auto` starts at the element's
//! **static position** (CSS 2.1 §10.3.7 / §10.6.4): phase-1 block,
//! inline and flex layout record it through [`record_static_position`]
//! at the point in the flow where the element's hypothetical box would
//! have gone, and `place::compute_placed_rect` reads it back.

//!
//! A `::before` / `::after` is positioned as an element is (CSS Pseudo 4
//! §2): an absolutely or fixed positioned one is a box phase 2 places
//! with the elements (`pseudo`), a relatively positioned or sticky one
//! is laid out in flow and moved once the document is laid out
//! (`pseudo_offsets`).
//!
//! - `static_pos` — the static position, recorded in phase 1.
//! - `relative` — the relative shift.
//! - `place` — phase-2 placement of absolute / fixed boxes.
//! - `pseudo` — an absolute / fixed `::before` / `::after`: its box and
//!   its static position.
//! - `pseudo_offsets` — the move of a relative / sticky `::before` /
//!   `::after`.
//! - `anchor` — anchor positioning: a box placed against its anchors.
//! - `axis` — one-axis size and offset resolvers.
//! - `containing` — the containing block of an absolutely positioned
//!   box, element or pseudo-element.

mod anchor;
mod axis;
mod containing;
mod place;
mod pseudo;
mod pseudo_offsets;
mod relative;
mod static_pos;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position};

pub(crate) use anchor::visibility::hidden as position_hidden;
pub(super) use axis::axis_position_anchored;
pub(super) use containing::{
    absolute_containing_block, containing_ancestor, fixed_containing_ancestor,
};
pub(super) use place::place_positioned;
pub(super) use pseudo_offsets::offset_in_flow_pseudos;
pub(super) use relative::{apply_relative_shift, relative_offset};
pub(super) use static_pos::{
    out_of_flow_positioned_children, record_static_position, record_static_positions_in_ifc,
    static_anchors, static_position_in_ifc,
};

/// Resolve the containing block rect for `id`, given the root
/// viewport. The element's own `position` decides:
///
/// - `Fixed` / `Absolute` → [`absolute_containing_block`], the walk
///   positioned pseudo-elements share; the viewport for an element in
///   the top layer (CSS Position 4).
/// - `Relative` / `Static` → returns the parent's content area
///   (or viewport if no parent), matching the in-flow position.
///   (Used by phase-2 callers that ask "where would this be in
///   flow?" for static-position resolution; see §5 of the spec.)
pub(crate) fn containing_block(dom: &Dom<TuiExt>, id: NodeId, viewport: LayoutRect) -> LayoutRect {
    let position = computed_position(dom, id);

    if matches!(position, Position::Absolute | Position::Fixed) {
        // CSS Position 4: a top-layer element's containing block is the
        // initial containing block — rdom's viewport — whatever its
        // ancestors (its `position` is `absolute` or `fixed`, the
        // cascade's `finalize_top_layer`).
        if dom.is_in_top_layer(id) {
            return viewport;
        }
        let c = dom
            .node(id)
            .ext()
            .and_then(|e| e.computed.clone())
            .unwrap_or_else(|| std::rc::Rc::new(crate::style::ComputedStyle::initial()));
        return absolute_containing_block(dom, parent_id(dom, id), &c, viewport);
    }

    // Static / Relative: containing block = parent's layout rect
    // (or viewport if no parent in the layout tree yet).
    parent_id(dom, id)
        .and_then(|p| layout_rect(dom, p))
        .unwrap_or(viewport)
}

pub(in crate::render::layout_pass) fn computed_position(dom: &Dom<TuiExt>, id: NodeId) -> Position {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        // A box-less element has no box to position (CSS Display 3 §2.5).
        .filter(|c| c.display != crate::layout::Display::Contents)
        .map(|c| c.position)
        .unwrap_or_default()
}

pub(in crate::render::layout_pass) fn layout_rect(
    dom: &Dom<TuiExt>,
    id: NodeId,
) -> Option<LayoutRect> {
    dom.node(id).ext().map(|e| e.layout)
}

/// `id`'s box parent (`box_tree::box_parent`: through `display:
/// contents` ancestors, which have no box to be a containing block).
pub(in crate::render::layout_pass) fn parent_id(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    crate::render::box_tree::box_parent(dom, id)
}

#[cfg(test)]
mod tests;
