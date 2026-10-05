//! An inline run's anonymous block box (CSS 2.1 §9.2.1.1): the run of a
//! block container's inline-level children packed as one inline
//! formatting context at the container's content width, its lines beside
//! the floats of the formatting context and its own floats placed there
//! (§9.5), its atoms laid out at their fragments' rects.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use super::Run;
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::LayoutRect;
use crate::render::inline::{RunPseudos, pack_run};
use crate::render::layout_pass::layout_node;

/// Where an inline run's box goes: its top-left corner and width
/// (`at.height` unused), and the top of the containing block's content
/// box (the edge `margin-trim` trims a float at).
pub(super) struct RunPlace {
    pub(super) at: LayoutRect,
    pub(super) content_top: i32,
}

/// Lay `run`, an inline run of `id`'s children with the host
/// pseudo-elements `pseudos`, out as an anonymous block box at `place`:
/// its lines packed (the floats it meets placed and laid out), the atoms
/// on them laid out at their rects — which writes their layout rects, so
/// hit-testing descends into them (`<form><button>Go</button></form>`
/// routes a click to the button), and lays out their subtrees — and the
/// out-of-flow boxes anchored on its children (`static_before`) given
/// their static positions in its lines. Its box is as tall as its lines.
pub(super) fn lay_out(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    run: &Run,
    pseudos: RunPseudos,
    place: RunPlace,
    static_before: &HashMap<NodeId, Vec<NodeId>>,
) -> AnonymousIfc {
    let width = place.at.width;
    let (mut inline_layout, floats) =
        crate::render::layout_pass::float::with_area(dom, |dom, area| {
            let mut ex = crate::render::layout_pass::float::lines::InlineFloats::new(
                dom,
                area,
                place.at,
                place.content_top,
            );
            let layout = pack_run(dom, id, &run.children, pseudos, width, Some(&mut ex));
            (layout, ex.into_placed())
        });
    crate::render::layout_pass::generated_atoms::lay_out(dom, &mut inline_layout);
    let rect = LayoutRect::new(place.at.x, place.at.y, width, inline_layout.height());
    for (atom, at) in crate::render::inline::atomic_placements(&inline_layout, rect) {
        layout_node(dom, atom, at, width);
    }
    for (f, placed) in floats {
        crate::render::layout_pass::float::lay_out(dom, id, f, placed, width);
    }
    for c in run.children.iter().filter_map(|c| c.node()) {
        for &n in static_before.get(&c).into_iter().flatten() {
            let (x, y) = crate::render::layout_pass::positioning::static_position_in_ifc(
                dom,
                id,
                n,
                &inline_layout,
                rect,
            );
            crate::render::layout_pass::positioning::record_static_position(dom, n, x, y);
        }
    }
    AnonymousIfc::new(rect, inline_layout, run.child_range, None)
}
