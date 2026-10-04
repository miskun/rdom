//! Positioning — containing-block resolution + phase-2 placement
//! for `position: absolute | fixed` elements (M2).
//!
//! Containing-block resolution:
//!
//! - `position: fixed` → always the initial containing block (the
//!   root viewport).
//! - `position: absolute` → the nearest ancestor whose
//!   `position` is `relative | absolute | fixed`, or the viewport
//!   if none.
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
//! - `static_pos` — the static position, recorded in phase 1.
//! - `relative` — the relative shift.
//! - `place` — phase-2 placement of absolute / fixed elements.
//! - `axis` — one-axis size and offset resolvers, shared with
//!   positioned pseudo-elements.

mod axis;
mod place;
mod relative;
mod static_pos;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position};

pub(super) use axis::{axis_position_anchored, axis_position_relative_shift};
pub(super) use place::{place_positioned, resolve_size_axis};
pub(super) use relative::apply_relative_shift;
pub(super) use static_pos::{
    out_of_flow_positioned_children, record_static_position, record_static_positions_in_ifc,
    static_anchors, static_position_in_ifc,
};

/// Resolve the containing block rect for `id`, given the root
/// viewport. The element's own `position` decides:
///
/// - `Fixed` → viewport.
/// - `Absolute` → ancestor walk; first positioned (relative,
///   absolute, fixed) ancestor's layout rect; viewport on miss.
/// - `Relative` / `Static` → returns the parent's content area
///   (or viewport if no parent), matching the in-flow position.
///   (Used by phase-2 callers that ask "where would this be in
///   flow?" for static-position resolution; see §5 of the spec.)
pub(crate) fn containing_block(dom: &Dom<TuiExt>, id: NodeId, viewport: LayoutRect) -> LayoutRect {
    let position = computed_position(dom, id);

    if position == Position::Fixed {
        return viewport;
    }

    if position == Position::Absolute {
        let mut cur = parent_id(dom, id);
        while let Some(p) = cur {
            let pp = computed_position(dom, p);
            if matches!(
                pp,
                Position::Relative | Position::Absolute | Position::Fixed
            ) {
                return layout_rect(dom, p).unwrap_or(viewport);
            }
            cur = parent_id(dom, p);
        }
        return viewport;
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
        .map(|c| c.position)
        .unwrap_or_default()
}

pub(in crate::render::layout_pass) fn layout_rect(
    dom: &Dom<TuiExt>,
    id: NodeId,
) -> Option<LayoutRect> {
    dom.node(id).ext().map(|e| e.layout)
}

pub(in crate::render::layout_pass) fn parent_id(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    dom.node(id).parent_node().map(|p| p.id())
}

#[cfg(test)]
mod tests;
