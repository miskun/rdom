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
    LayerEntry, Layers, collect_layers, creates_stacking_context, for_each_atom_shadow,
    is_positioned, paints_atomically,
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
/// in-flow content. An atomic box (`parent`'s flex item, an inline
/// block) is a paint unit of its own: its shadows paint whole, then its
/// in-flow boxes' shadows (its background phase), then its content.
fn paint_plain(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let atomic = dom
        .node(id)
        .computed()
        .is_some_and(|c| paints_atomically(dom, parent, c));
    let shadows = if atomic {
        Shadows::Whole
    } else {
        Shadows::UnderText
    };
    let Some(frame) = paint_box(dom, id, buf, clip, shadows) else {
        return;
    };
    if atomic {
        for_each_atom_shadow(dom, id, frame.children_clip, &mut |e| {
            shadow::paint_backdrop_shadow(dom, &e, buf);
        });
    }
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
    children_of(dom, id, id, buf, clip, viewport);
}

/// [`recurse_children`] over the child nodes of `node`, whose box
/// parent is `id` (`node` itself, or a `display: contents` element in
/// `id`, whose children are `id`'s in the box tree — CSS Display 3
/// §2.5; it paints nothing of its own).
fn children_of(
    dom: &Dom<TuiExt>,
    node: NodeId,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    // A flex container's items paint in order-modified document order
    // (CSS Flexbox §5.4).
    let kids = if node == id {
        crate::render::box_tree::paint_order_children(dom, id)
    } else {
        dom.node(node).child_nodes().map(|c| c.id()).collect()
    };
    for cid in kids {
        let child = dom.node(cid);
        match child.node_type() {
            NodeType::Element if crate::render::box_tree::is_contents(dom, cid) => {
                children_of(dom, cid, id, buf, clip, viewport);
            }
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
                if orphan_inline(dom, cid) || in_a_line(dom, id, cid) {
                    continue;
                }
                paint_in_flow(dom, id, cid, buf, clip, viewport);
            }
            NodeType::Fragment => recurse_children(dom, cid, buf, clip, viewport),
            // Text is consumed by the parent's inline pass; comments and
            // any later node kind (`NodeType` is `#[non_exhaustive]`)
            // do not render.
            _ => {}
        }
    }
}

/// Paint the in-flow element `id`, a child of `parent`, in place: a
/// positioned one is skipped (its stacking context's layers paint it),
/// one that establishes a stacking context paints as one, any other as
/// a plain box ([`paint_plain`]).
fn paint_in_flow(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    match dom.node(id).computed() {
        Some(c) if is_positioned(c) => {}
        Some(c) if creates_stacking_context(c) => {
            paint_stacking_context(dom, id, buf, clip, viewport);
        }
        _ => paint_plain(dom, parent, id, buf, clip, viewport),
    }
}

/// Paint the atomic inline box `atom` at its turn in its line (CSS 2.1
/// Appendix E, 7.2.1.4.1.1: an inline block paints atomically, as if it
/// created a stacking context — its outer shadows, background, border,
/// then its content), over the line content painted before it. The line
/// is its one painter: its parent's content paint skips it
/// ([`in_a_line`]).
pub(super) fn paint_line_atom(
    dom: &Dom<TuiExt>,
    atom: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let Some(parent) = crate::render::box_tree::box_parent(dom, atom) else {
        return;
    };
    paint_in_flow(dom, parent, atom, buf, clip, viewport);
}

/// True when the element child `child` of `parent` is an atom of one of
/// `parent`'s lines — an inline block in a block container, packed into
/// an inline formatting context (`parent`'s own or an anonymous block
/// box's) — so the line paints it ([`paint_line_atom`]), not
/// [`recurse_children`]. A flex container's inline-block children are
/// flex items (blockified, CSS Flexbox §4) and paint as boxes.
fn in_a_line(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    let p = dom.node(parent);
    p.node_type() == NodeType::Element
        && p.computed().is_some_and(|c| c.flow.is_block_flow())
        && dom
            .node(child)
            .computed()
            .is_some_and(|c| c.display == Display::InlineBlock)
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
/// neither an orphan inline nor an atom of one of its lines. `stacking::collect_layers` asks, to gather the
/// boxes whose shadows paint in the background phase.
pub(crate) fn paints_child_box(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    let p = dom.node(parent);
    let element = p.node_type() == NodeType::Element;
    if element && (p.ext().is_some_and(|e| e.canvas_paint.is_some()) || is_ifc_block(dom, parent)) {
        return false;
    }
    !orphan_inline(dom, child) && !in_a_line(dom, parent, child)
}
