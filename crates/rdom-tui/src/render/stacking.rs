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
//! walk that stops at nested contexts, and with them the in-flow boxes
//! whose outer `box-shadow`s belong to the background phase of 3 (CSS
//! 2.1 Appendix E step 4 paints block backgrounds — and box shadows,
//! Backgrounds 3 §7.2 — before any inline content; see
//! `paint_pass::shadow`). An atomic box — an inline block, an inline
//! flex container, a flex item (which paints exactly as an inline
//! block, Flexbox §5.4) — paints as if it created a stacking context
//! whose positioned descendants still belong to this one (Appendix E,
//! step 7.2.1.4.1.1): its in-flow boxes' shadows are its own background
//! phase ([`for_each_atom_shadow`]), not this context's. Each entry carries the clip that
//! applies to it: CSS 2.1 §11.1.1 — an overflow ancestor clips a
//! positioned descendant only when the descendant's containing block is
//! that ancestor or lies inside it, so an `absolute` box takes the clip
//! in effect at its containing block, a `fixed` one the viewport, and
//! `relative` / `sticky` boxes the clip of their parent's content.
//!
//! Paint walks the layers forward; hit-testing walks them backward.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Flow, Overflow, Position, ZIndex};
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

impl LayerEntry {
    /// The paint unit a `z-index: auto` entry is: its in-flow boxes'
    /// shadows are [`Layers::shadows_of`] this (the context itself is
    /// unit 0).
    pub(crate) fn unit(&self) -> usize {
        self.order + 1
    }
}

/// An in-flow box with an outer shadow, which paints in the background
/// phase of its paint unit: the stacking context (unit 0), or the
/// `z-index: auto` positioned box it lies in, which paints as if it
/// were one ([`LayerEntry::unit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShadowEntry {
    pub id: NodeId,
    /// The clip the box paints into.
    pub clip: Rect,
    pub unit: usize,
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
    /// In-flow boxes with an outer shadow, by unit, in tree order.
    pub shadows: Vec<ShadowEntry>,
}

impl Layers {
    /// The shadowed in-flow boxes of paint unit `unit`, in tree order.
    pub(crate) fn shadows_of(&self, unit: usize) -> &[ShadowEntry] {
        let start = self.shadows.partition_point(|s| s.unit < unit);
        let end = self.shadows.partition_point(|s| s.unit <= unit);
        &self.shadows[start..end]
    }
}

/// CSS "positioned": any `position` other than `static`. A box-less
/// (`display: contents`) element is never positioned: it has no box
/// for `position` to apply to (CSS Display 3 §2.5).
pub(crate) fn is_positioned(c: &ComputedStyle) -> bool {
    c.position != Position::Static && c.display != Display::Contents
}

/// Does the in-flow element `c` (a child of `parent`) paint
/// atomically — as an inline block does, as if it created a stacking
/// context (CSS 2.1 Appendix E)? Inline blocks and inline flex
/// containers do, and so do flex items (CSS Flexbox §5.4: they paint
/// exactly as inline blocks). The children of the document root are
/// block boxes for paint (rdom lays them out as flex items, a
/// documented divergence; a browser's `<body>` children are blocks).
pub(crate) fn paints_atomically(dom: &Dom<TuiExt>, parent: NodeId, c: &ComputedStyle) -> bool {
    if c.display == Display::InlineBlock || (c.display == Display::Inline && c.flow == Flow::Flex) {
        return true;
    }
    // A fragment or box-less child is laid out in the element above it.
    let mut p = dom.node(parent);
    while p.node_type() == NodeType::Fragment || crate::render::box_tree::is_contents(dom, p.id()) {
        match p.parent_node() {
            Some(up) => p = up,
            None => return false,
        }
    }
    p.node_type() == NodeType::Element
        && p.ext()
            .and_then(|e| e.computed.as_ref())
            .is_some_and(|pc| pc.flow == Flow::Flex)
}

/// Does an element with this style establish a stacking context?
/// (The document root always does.)
pub(crate) fn creates_stacking_context(c: &ComputedStyle) -> bool {
    // No box, no stacking context (CSS Display 3 §2.5).
    c.display != Display::Contents
        && ((is_positioned(c) && !matches!(c.z_index, ZIndex::Auto)) || c.opacity < 1.0)
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
    let padding_box = crate::layout::compute_padding_box(outer, c.border);
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
    let chain = vec![Frame {
        positioned: root_positioned,
        content_clip,
    }];
    let mut walk = Walk {
        dom,
        viewport,
        chain,
        layers: &mut layers,
        order: 0,
    };
    walk.children(root, root, Some(0));
    layers.negative.sort_by_key(|e| (e.z, e.order));
    layers.positive.sort_by_key(|e| (e.z, e.order));
    // Stable: tree order within each unit.
    layers.shadows.sort_by_key(|s| s.unit);
    layers
}

/// The state of [`collect_layers`]' walk.
struct Walk<'a> {
    dom: &'a Dom<TuiExt>,
    viewport: Rect,
    chain: Vec<Frame>,
    layers: &'a mut Layers,
    order: usize,
}

impl Walk<'_> {
    /// Walk the children of `id`; `box_parent` is the element whose
    /// content paint reaches them (`id`, or the element above a
    /// fragment), `unit` the paint unit they belong to — `None` inside
    /// an atomic box, whose own paint gathers its in-flow shadows
    /// ([`for_each_atom_shadow`]).
    fn children(&mut self, id: NodeId, box_parent: NodeId, unit: Option<usize>) {
        let dom = self.dom;
        let viewport = self.viewport;
        for child in dom.node(id).child_nodes() {
            let cid = child.id();
            match child.node_type() {
                // A box-less element's children are its parent box's
                // (CSS Display 3 §2.5), as a fragment's are.
                NodeType::Fragment => {
                    self.children(cid, box_parent, unit);
                    continue;
                }
                NodeType::Element if crate::render::box_tree::is_contents(dom, cid) => {
                    self.children(cid, box_parent, unit);
                    continue;
                }
                NodeType::Element => {}
                _ => continue,
            }
            let current = *self
                .chain
                .last()
                .expect("the context root frame is always present");
            let Some(c) = child.ext().and_then(|e| e.computed.as_ref()) else {
                // Not cascaded: an in-flow box with nothing to clip.
                self.chain.push(Frame {
                    positioned: false,
                    content_clip: current.content_clip,
                });
                self.children(cid, cid, unit);
                self.chain.pop();
                continue;
            };
            if c.display == Display::None {
                continue;
            }
            if is_positioned(c) {
                let clip = match c.position {
                    Position::Fixed => viewport,
                    Position::Absolute => self
                        .chain
                        .iter()
                        .rev()
                        .find(|f| f.positioned)
                        .map_or(self.chain[0].content_clip, |f| f.content_clip),
                    Position::Relative | Position::Sticky | Position::Static => {
                        current.content_clip
                    }
                };
                let context = creates_stacking_context(c);
                let z = match c.z_index {
                    ZIndex::Auto => 0,
                    ZIndex::Value(n) => n,
                };
                let entry = LayerEntry {
                    id: cid,
                    z,
                    order: self.order,
                    context,
                    clip,
                };
                self.order += 1;
                match z {
                    _ if !context => self.layers.zero_auto.push(entry),
                    z if z < 0 => self.layers.negative.push(entry),
                    0 => self.layers.zero_auto.push(entry),
                    _ => self.layers.positive.push(entry),
                }
                if !context {
                    // `z-index: auto`: its positioned descendants belong
                    // to this context, clipped by its own content clip;
                    // its in-flow boxes to its own paint unit.
                    self.chain.push(Frame {
                        positioned: true,
                        content_clip: children_clip(dom, cid, c, clip),
                    });
                    self.children(cid, cid, Some(entry.unit()));
                    self.chain.pop();
                }
            } else if creates_stacking_context(c) {
                // `opacity < 1` on an in-flow box: painted atomically in
                // place; nothing inside it belongs to this context.
            } else {
                // An atomic box paints its own shadow whole at its turn,
                // and its in-flow boxes' in its own background phase;
                // only its positioned descendants are this context's.
                let atomic = paints_atomically(dom, box_parent, c);
                if let Some(unit) = unit
                    && !atomic
                    && casts_backdrop_shadow(dom, box_parent, cid, c)
                {
                    self.layers.shadows.push(ShadowEntry {
                        id: cid,
                        clip: current.content_clip,
                        unit,
                    });
                }
                self.chain.push(Frame {
                    positioned: false,
                    content_clip: children_clip(dom, cid, c, current.content_clip),
                });
                self.children(cid, cid, if atomic { None } else { unit });
                self.chain.pop();
            }
        }
    }
}

/// Does the in-flow, non-atomic element `id` (style `c`, reached
/// through `box_parent`'s content paint) have an outer shadow that
/// paints in its unit's background phase?
fn casts_backdrop_shadow(
    dom: &Dom<TuiExt>,
    box_parent: NodeId,
    id: NodeId,
    c: &ComputedStyle,
) -> bool {
    c.box_shadow.iter().any(|s| !s.inset)
        && crate::render::paint_pass::paints_child_box(dom, box_parent, id)
}

/// The background phase of the atomic box `atom`
/// ([`paints_atomically`]): call `f` with each of its in-flow,
/// non-atomic boxes that casts an outer shadow, in tree order.
/// `content_clip` is the clip `atom`'s content paints into. The walk
/// stops at positioned boxes and stacking contexts (they belong to the
/// enclosing context's layers) and at nested atomic boxes (their own
/// units), so each box is visited by one unit only; it allocates
/// nothing.
pub(crate) fn for_each_atom_shadow(
    dom: &Dom<TuiExt>,
    atom: NodeId,
    content_clip: Rect,
    f: &mut impl FnMut(ShadowEntry),
) {
    atom_shadows_in(dom, atom, atom, content_clip, f);
}

fn atom_shadows_in(
    dom: &Dom<TuiExt>,
    id: NodeId,
    box_parent: NodeId,
    clip: Rect,
    f: &mut impl FnMut(ShadowEntry),
) {
    for child in dom.node(id).child_nodes() {
        let cid = child.id();
        match child.node_type() {
            NodeType::Fragment => {
                atom_shadows_in(dom, cid, box_parent, clip, f);
                continue;
            }
            NodeType::Element if crate::render::box_tree::is_contents(dom, cid) => {
                atom_shadows_in(dom, cid, box_parent, clip, f);
                continue;
            }
            NodeType::Element => {}
            _ => continue,
        }
        let Some(c) = child.ext().and_then(|e| e.computed.as_ref()) else {
            atom_shadows_in(dom, cid, cid, clip, f);
            continue;
        };
        if c.display == Display::None
            || is_positioned(c)
            || creates_stacking_context(c)
            || paints_atomically(dom, box_parent, c)
        {
            continue;
        }
        if casts_backdrop_shadow(dom, box_parent, cid, c) {
            f(ShadowEntry {
                id: cid,
                clip,
                unit: 0,
            });
        }
        atom_shadows_in(dom, cid, cid, children_clip(dom, cid, c, clip), f);
    }
}
