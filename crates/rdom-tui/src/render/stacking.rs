//! Stacking order (CSS 2.1 Appendix E), shared by paint and hit-test.
//!
//! A stacking context is rooted at the document root, at every
//! positioned element with a numeric `z-index`, and at every element
//! with `opacity < 1`. Inside one context the order is:
//!
//! 1. the root's own background and border,
//! 2. child contexts with negative `z-index`, ascending,
//! 3. the root's in-flow content in tree order (a non-positioned
//!    nested context paints atomically in its place),
//! 4. positioned descendants with `z-index: auto | 0` in tree order —
//!    an `auto` one as a plain box whose own positioned descendants
//!    belong to this context, a `0` one as a child context,
//! 5. child contexts with positive `z-index`, ascending.
//!
//! [`collect_layers`] gathers 2, 4 and 5 for one context in a single
//! walk that stops at nested contexts. Each entry carries the clip that
//! applies to it: CSS 2.1 §11.1.1 — an overflow ancestor clips a
//! positioned descendant only when the descendant's containing block is
//! that ancestor or lies inside it, so an `absolute` box takes the clip
//! in effect at its containing block, a `fixed` one the viewport, and
//! `relative` / `sticky` boxes the clip of their parent's content.
//!
//! Paint walks the layers forward; hit-testing walks them backward.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Overflow, Position, ZIndex};
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::paint_pass::layout_rect_to_grid;
use crate::style::ComputedStyle;

/// One positioned descendant of a stacking context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerEntry {
    pub id: NodeId,
    /// `z-index`, with `auto` as 0.
    pub z: i16,
    /// Tree order among the context's entries; the tie-break.
    pub order: usize,
    /// Paints as a child stacking context (numeric `z-index` or
    /// `opacity < 1`) rather than as a plain positioned box.
    pub context: bool,
    /// The clip this entry paints into.
    pub clip: Rect,
}

/// The positioned descendants of one stacking context, by layer.
#[derive(Debug, Default)]
pub(crate) struct Layers {
    /// Child contexts with negative `z-index`, ascending `(z, order)`.
    pub negative: Vec<LayerEntry>,
    /// `z-index: auto | 0`, in tree order.
    pub zero_auto: Vec<LayerEntry>,
    /// Child contexts with positive `z-index`, ascending `(z, order)`.
    pub positive: Vec<LayerEntry>,
}

/// CSS "positioned": any `position` other than `static`.
pub(crate) fn is_positioned(c: &ComputedStyle) -> bool {
    c.position != Position::Static
}

/// Does an element with this style establish a stacking context?
/// (The document root always does.)
pub(crate) fn creates_stacking_context(c: &ComputedStyle) -> bool {
    (is_positioned(c) && !matches!(c.z_index, ZIndex::Auto)) || c.opacity < 1.0
}

/// The clip `id`'s content paints into, given the clip `id` itself
/// paints into: the padding box (CSS Overflow 3 §3) when either
/// overflow axis clips, `clip` unchanged otherwise.
pub(crate) fn children_clip(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle, clip: Rect) -> Rect {
    let clips =
        !matches!(c.overflow_x, Overflow::Visible) || !matches!(c.overflow_y, Overflow::Visible);
    if !clips {
        return clip;
    }
    let outer = dom.node(id).layout_rect().unwrap_or_default();
    let padding_box = rdom_style::layout::compute_padding_box(outer, c.border);
    layout_rect_to_grid(padding_box, clip).unwrap_or_else(|| Rect::new(clip.x, clip.y, 0, 0))
}

/// An ancestor on the walk from the context root down: whether it is
/// positioned (a containing-block candidate) and the clip its content
/// paints into.
#[derive(Clone, Copy)]
struct Frame {
    positioned: bool,
    content_clip: Rect,
}

/// Gather the positioned descendants that belong to the stacking
/// context rooted at `root`. `content_clip` is the clip the root's
/// content paints into; `viewport` the document's clip (the clip of
/// `position: fixed` descendants).
///
/// An `absolute` descendant whose containing block lies above `root`
/// takes `content_clip`: exact for the document root (its containing
/// block is the viewport) and the nearest available clip for an
/// `opacity` context, whose ancestors this walk does not see.
pub(crate) fn collect_layers(
    dom: &Dom<TuiExt>,
    root: NodeId,
    content_clip: Rect,
    viewport: Rect,
) -> Layers {
    let mut layers = Layers::default();
    let root_positioned = dom
        .node(root)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| is_positioned(c));
    let mut chain = vec![Frame {
        positioned: root_positioned,
        content_clip,
    }];
    let mut order = 0;
    walk(dom, root, viewport, &mut chain, &mut layers, &mut order);
    layers.negative.sort_by_key(|e| (e.z, e.order));
    layers.positive.sort_by_key(|e| (e.z, e.order));
    layers
}

fn walk(
    dom: &Dom<TuiExt>,
    id: NodeId,
    viewport: Rect,
    chain: &mut Vec<Frame>,
    layers: &mut Layers,
    order: &mut usize,
) {
    for child in dom.node(id).child_nodes() {
        let cid = child.id();
        match child.node_type() {
            NodeType::Fragment => {
                walk(dom, cid, viewport, chain, layers, order);
                continue;
            }
            NodeType::Element => {}
            _ => continue,
        }
        let current = *chain
            .last()
            .expect("the context root frame is always present");
        let Some(c) = child.ext().and_then(|e| e.computed.as_ref()) else {
            // Not cascaded: an in-flow box with nothing to clip.
            chain.push(Frame {
                positioned: false,
                content_clip: current.content_clip,
            });
            walk(dom, cid, viewport, chain, layers, order);
            chain.pop();
            continue;
        };
        if c.display == Display::None {
            continue;
        }
        if is_positioned(c) {
            let clip = match c.position {
                Position::Fixed => viewport,
                Position::Absolute => chain
                    .iter()
                    .rev()
                    .find(|f| f.positioned)
                    .map_or(chain[0].content_clip, |f| f.content_clip),
                Position::Relative | Position::Sticky | Position::Static => current.content_clip,
            };
            let context = creates_stacking_context(c);
            let z = match c.z_index {
                ZIndex::Auto => 0,
                ZIndex::Value(n) => n,
            };
            let entry = LayerEntry {
                id: cid,
                z,
                order: *order,
                context,
                clip,
            };
            *order += 1;
            match z {
                _ if !context => layers.zero_auto.push(entry),
                z if z < 0 => layers.negative.push(entry),
                0 => layers.zero_auto.push(entry),
                _ => layers.positive.push(entry),
            }
            if !context {
                // `z-index: auto`: its positioned descendants belong to
                // this context, clipped by its own content clip.
                chain.push(Frame {
                    positioned: true,
                    content_clip: children_clip(dom, cid, c, clip),
                });
                walk(dom, cid, viewport, chain, layers, order);
                chain.pop();
            }
        } else if creates_stacking_context(c) {
            // `opacity < 1` on an in-flow box: painted atomically in
            // place; nothing inside it belongs to this context.
        } else {
            chain.push(Frame {
                positioned: false,
                content_clip: children_clip(dom, cid, c, current.content_clip),
            });
            walk(dom, cid, viewport, chain, layers, order);
            chain.pop();
        }
    }
}
