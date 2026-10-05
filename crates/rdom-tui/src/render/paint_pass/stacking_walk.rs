//! The stacking-context walk (CSS 2.1 Appendix E): a context's root
//! box, its layers around its in-flow content, the in-flow children in
//! tree order, and the background phase of each paint unit (the outer
//! shadows of its in-flow boxes, `shadow`). The per-box paint is
//! `box_paint`'s.

use rdom_core::{Dom, NodeId, NodeType};

use super::box_paint::{paint_box, paint_content};
use super::group;
use super::shadow::{self, Shadows};
use crate::ext::TuiExt;
use crate::layout::Display;
use crate::node::TuiNodeExt;
use crate::render::layout_pass::is_ifc_block;
use crate::render::stacking::{
    LayerEntry, Layers, collect_layers, creates_stacking_context, is_positioned,
};
use crate::render::{Buffer, Rect};

/// Paint `root` and everything stacked inside it in CSS 2.1 Appendix E
/// order: the root's own box, child contexts with negative `z-index`,
/// the root's in-flow content, positioned descendants with `z-index:
/// auto | 0` in tree order, child contexts with positive `z-index`.
///
/// `clip` is the region this context paints into; `viewport` the
/// document's clip, which `position: fixed` descendants clip to.
pub(super) fn paint_stacking_context(
    dom: &Dom<TuiExt>,
    root: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    // CSS `opacity` is group opacity: the subtree paints into a layer
    // at full opacity and the layer composites back at the element's
    // alpha, so nested opacities multiply and every paint inside —
    // backgrounds, glyphs, borders — blends once against the backdrop
    // (OPACITY-1; see `group`).
    let alpha = dom.node(root).computed().map_or(1.0, |c| c.opacity);
    if alpha < 1.0 && dom.node(root).node_type() == NodeType::Element {
        group::paint_group(dom, root, buf, alpha, |layer| {
            paint_stacking_context_body(dom, root, layer, clip, viewport);
        });
        return;
    }
    paint_stacking_context_body(dom, root, buf, clip, viewport);
}

fn paint_stacking_context_body(
    dom: &Dom<TuiExt>,
    root: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    if dom.node(root).node_type() == NodeType::Fragment {
        // The document root: no box of its own.
        let layers = collect_layers(dom, root, clip, viewport);
        paint_layers(dom, &layers, &layers.negative, buf, viewport);
        shadow::paint_backdrop_shadows(dom, layers.shadows_of(0), buf);
        recurse_children(dom, root, buf, clip, viewport);
        paint_layers(dom, &layers, &layers.zero_auto, buf, viewport);
        paint_layers(dom, &layers, &layers.positive, buf, viewport);
        return;
    }
    let Some(frame) = paint_box(dom, root, buf, clip, Shadows::Whole) else {
        return;
    };
    let layers = collect_layers(dom, root, frame.children_clip, viewport);
    paint_layers(dom, &layers, &layers.negative, buf, viewport);
    shadow::paint_backdrop_shadows(dom, layers.shadows_of(0), buf);
    paint_content(dom, root, buf, clip, viewport, &frame);
    paint_layers(dom, &layers, &layers.zero_auto, buf, viewport);
    paint_layers(dom, &layers, &layers.positive, buf, viewport);
}

fn paint_layers(
    dom: &Dom<TuiExt>,
    layers: &Layers,
    entries: &[LayerEntry],
    buf: &mut Buffer,
    viewport: Rect,
) {
    for e in entries {
        if e.context {
            paint_stacking_context(dom, e.id, buf, e.clip, viewport);
            continue;
        }
        // A `z-index: auto` positioned box paints as if it were a
        // context: its box, its in-flow boxes' shadows, its content.
        let Some(frame) = paint_box(dom, e.id, buf, e.clip, Shadows::Whole) else {
            continue;
        };
        shadow::paint_backdrop_shadows(dom, layers.shadows_of(e.unit()), buf);
        paint_content(dom, e.id, buf, e.clip, viewport, &frame);
    }
}

/// Paint an in-flow element as a plain box: its own box, then its
/// in-flow content.
fn paint_plain(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect, viewport: Rect) {
    let Some(frame) = paint_box(dom, id, buf, clip, Shadows::UnderText) else {
        return;
    };
    paint_content(dom, id, buf, clip, viewport, &frame);
}

/// Paint the in-flow element children of `id` in tree order. Positioned
/// children are skipped — they paint from the enclosing stacking
/// context's layers — and a child that establishes a stacking context
/// without being positioned (`opacity < 1`) paints atomically in place.
pub(super) fn recurse_children(
    dom: &Dom<TuiExt>,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    for child in dom.node(id).child_nodes() {
        let cid = child.id();
        match child.node_type() {
            NodeType::Element => {
                // Display:inline children outside an IFC context are
                // a cascade error in CSS — but in rdom, a flex
                // container with at least one `display: inline-block`
                // child (e.g. `<button>`) escapes IFC and treats all
                // its children as flex items (see
                // `layout_pass/ifc.rs`'s `is_ifc_block`). In that
                // case an inline-display child gets a real layout
                // rect plus its own `inline_layout` from the pure-
                // text-leaf branch in flex layout — and must paint
                // through the normal element path.
                //
                // The original suppression still applies to "truly
                // orphaned" inline elements (no layout rect, no
                // inline_layout) — keep skipping those so they don't
                // paint as zero-sized blocks at (0,0).
                if orphan_inline(dom, cid) {
                    continue;
                }
                match child.ext().and_then(|e| e.computed.as_ref()) {
                    Some(c) if is_positioned(c) => continue,
                    Some(c) if creates_stacking_context(c) => {
                        paint_stacking_context(dom, cid, buf, clip, viewport);
                    }
                    _ => paint_plain(dom, cid, buf, clip, viewport),
                }
            }
            NodeType::Fragment => recurse_children(dom, cid, buf, clip, viewport),
            // Text is consumed by the parent's inline pass; comments and
            // any later node kind (`NodeType` is `#[non_exhaustive]`)
            // do not render.
            _ => {}
        }
    }
}

/// An inline element with no inline layout: outside an inline
/// formatting context it has no box to paint ([`recurse_children`]).
fn orphan_inline(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    let is_inline = node
        .ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| c.display == Display::Inline);
    is_inline && node.ext().and_then(|e| e.inline_layout.as_ref()).is_none()
}

/// True when the content paint of `parent` reaches its in-flow element
/// child `child` through [`recurse_children`] and paints it as a box:
/// `parent` has no canvas callback and is not an inline formatting
/// context (whose inline children `paint_ifc` paints), and `child` is
/// not an orphan inline. `stacking::collect_layers` asks, to gather the
/// boxes whose shadows paint in the background phase.
pub(crate) fn paints_child_box(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    let p = dom.node(parent);
    let element = p.node_type() == NodeType::Element;
    if element && (p.ext().is_some_and(|e| e.canvas_paint.is_some()) || is_ifc_block(dom, parent)) {
        return false;
    }
    !orphan_inline(dom, child)
}
