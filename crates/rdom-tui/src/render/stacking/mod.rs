//! Stacking order (CSS 2.1 Appendix E), shared by paint and hit-test.
//!
//! A stacking context is rooted at the document root, at every
//! positioned element with a numeric `z-index`, and at every element
//! with `opacity < 1`. Inside one context the order is:
//!
//! 1. the root's own background and border,
//! 2. child contexts with negative `z-index`, ascending,
//! 3. the root's in-flow content, in three phases (steps 4, 5 and 7):
//!    the backgrounds and borders of its in-flow block-level boxes in
//!    tree order (with their outer `box-shadow`s, Backgrounds 3 §7.2),
//!    then its floats in tree order, each atomically, then its inline
//!    content in tree order — text, and atomic boxes whole at their turn
//!    (a non-positioned nested context paints atomically in its place),
//! 4. positioned descendants with `z-index: auto | 0` in tree order —
//!    an `auto` one as a plain box whose own positioned descendants
//!    belong to this context, a `0` one as a child context,
//! 5. child contexts with positive `z-index`, ascending.
//!
//! A *paint unit* is a box that paints as if it created a stacking
//! context whose positioned descendants and real contexts still belong to
//! the enclosing one: the context root itself, a `z-index: auto`
//! positioned box (Appendix E step 8), a float (step 5) and an atomic box
//! — an inline block, an inline flex or grid container, a flex or grid
//! item, which paint exactly as inline blocks (Flexbox §5.4, Grid 2 §6.5;
//! step 7.2.1.4.1.1). Each unit paints its own box, then its in-flow
//! block-level boxes' backgrounds, its floats and its inline content.
//!
//! [`collect_layers`] (`collect`) gathers, in one walk of a context that
//! stops at nested contexts, the layers of 2, 4 and 5 and — for the
//! context root's unit and each `z-index: auto` unit — the boxes of the
//! background phase ([`BoxEntry`]) and the floats. A float and an atomic
//! box gather theirs when they paint ([`for_each_unit_box`], `unit`),
//! a walk of their own subtree that allocates nothing unless one holds a
//! float. Each entry carries the clip that applies to it: CSS 2.1 §11.1.1
//! — an overflow ancestor clips a positioned descendant only when the
//! descendant's containing block is that ancestor or lies inside it, so
//! an `absolute` box takes the clip in effect at its containing block, a
//! `fixed` one the viewport, and `relative` / `sticky` boxes the clip of
//! their parent's content.
//!
//! Paint walks the layers forward; hit-testing walks them backward.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Display, Position, ZIndex};
use crate::render::Rect;
use crate::render::paint_pass::layout_rect_to_grid;
use crate::style::ComputedStyle;

/// One positioned descendant of a stacking context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerEntry {
    pub id: NodeId,
    /// `z-index`, with `auto` as 0.
    pub z: i32,
    /// Tree order among the context's entries; the tie-break.
    pub order: usize,
    /// Paints as a child stacking context (numeric `z-index` or
    /// `opacity < 1`) rather than as a plain positioned box.
    pub context: bool,
    /// The clip this entry paints into.
    pub clip: Rect,
    /// A floated `::before` / `::after` rather than the element `id`: its
    /// index among `id`'s floated pseudo-elements (`TuiExt::floated_pseudos`,
    /// the boxes `id`'s formatting context run placed).
    pub generated: Option<usize>,
    /// For a float: the paint unit whose step 5 paints it — 0 for the
    /// context root, a `z-index: auto` entry's [`unit`](Self::unit). 0
    /// for any other entry.
    pub owner: usize,
}

impl LayerEntry {
    /// The paint unit a `z-index: auto` entry is: its in-flow block-level
    /// boxes are [`Layers::boxes_of`] this, its floats those
    /// [`Layers::floats_of`] it (the context itself is unit 0).
    pub(crate) fn unit(&self) -> usize {
        self.order + 1
    }
}

/// An in-flow block-level box of a paint unit, whose outer shadows,
/// background and border paint in the unit's background phase (CSS 2.1
/// Appendix E step 4) — before any inline content of the unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoxEntry {
    pub id: NodeId,
    /// A block-level `::before` / `::after` of `id` rather than `id`
    /// itself: its index among `id`'s anonymous boxes
    /// (`AnonymousIfc::generated`).
    pub generated: Option<usize>,
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
    /// Floats that are not positioned, in tree order (Appendix E step
    /// 5), each with the unit it belongs to.
    pub floats: Vec<LayerEntry>,
    /// Child contexts with positive `z-index`, ascending `(z, order)`.
    pub positive: Vec<LayerEntry>,
    /// The in-flow block-level boxes of the context's units, by unit, in
    /// tree order.
    pub boxes: Vec<BoxEntry>,
}

impl Layers {
    /// The background phase of paint unit `unit`: its in-flow block-level
    /// boxes, in tree order.
    pub(crate) fn boxes_of(&self, unit: usize) -> &[BoxEntry] {
        let start = self.boxes.partition_point(|s| s.unit < unit);
        let end = self.boxes.partition_point(|s| s.unit <= unit);
        &self.boxes[start..end]
    }

    /// The floats of paint unit `unit`, in tree order.
    pub(crate) fn floats_of(&self, unit: usize) -> impl Iterator<Item = &LayerEntry> {
        self.floats.iter().filter(move |f| f.owner == unit)
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
/// context (CSS 2.1 Appendix E)? Inline blocks and inline flex and grid
/// containers do, and so do flex items (CSS Flexbox §5.4: they paint
/// exactly as inline blocks) and grid items (CSS Grid 2 §6.5, the same
/// words). The children of the document root are
/// block boxes for paint (rdom lays them out as flex items, a
/// documented divergence; a browser's `<body>` children are blocks).
pub(crate) fn paints_atomically(dom: &Dom<TuiExt>, parent: NodeId, c: &ComputedStyle) -> bool {
    crate::render::box_tree::is_atomic_inline(c) || is_item_of(dom, parent)
}

/// Whether the in-flow children of `parent` are flex or grid items: the
/// element their boxes are laid out in — `parent`, or past box-less
/// elements the one above — is a flex or grid container. The cascade's
/// answer (`cascade::children_are_items`, what blockification reads), so
/// paint and computed `display` agree (C7G-MINOR: this walk also went
/// through a `Fragment`, which a tree only has at its root — inserting
/// one unwraps it).
fn is_item_of(dom: &Dom<TuiExt>, parent: NodeId) -> bool {
    dom.node(parent)
        .ext()
        .and_then(|e| e.computed.as_deref())
        .is_some_and(|c| crate::style::cascade::children_are_items(dom, Some(parent), c))
}

/// Does the element `c`, a child of `parent`, paint and hit from its
/// stacking context's layers rather than at its turn in its parent's
/// content? A positioned box does, and so does a z-indexed flex or grid
/// item ([`is_z_indexed_item`]). Every such box that is not positioned
/// establishes a stacking context ([`creates_stacking_context`]).
pub(crate) fn is_layered(dom: &Dom<TuiExt>, id: NodeId, parent: NodeId, c: &ComputedStyle) -> bool {
    is_positioned(c) || is_z_indexed_item(dom, parent, c) || is_float(dom, id)
}

/// Does the element `id` float (CSS 2.1 §9.5) — paint from its stacking
/// context's float layer (Appendix E step 5)? Layout's one answer,
/// `layout_pass::float::float_side`, which reads its box parent.
pub(crate) fn is_float(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    crate::render::layout_pass::float::float_side(dom, id).is_some()
}

/// Does the element `c`, a child of `parent`, establish a stacking
/// context? A positioned box with a `z-index` other than `auto`, a box
/// with `opacity` below 1, and a z-indexed flex or grid item. (The
/// document root always does.) The one answer the paint and hit walks
/// share with [`is_layered`] (C7G-STACKING-ONE).
pub(crate) fn creates_stacking_context(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    c: &ComputedStyle,
) -> bool {
    // No box, no stacking context (CSS Display 3 §2.5).
    c.display != Display::Contents
        && ((is_positioned(c) && !matches!(c.z_index, ZIndex::Auto))
            || c.opacity < 1.0
            || is_z_indexed_item(dom, parent, c))
}

/// A flex or grid item with a `z-index` other than `auto` (CSS Flexbox
/// §5.4, CSS Grid 2 §6.5: such a value "create[s] a stacking context even
/// if `position` is `static`", ordered as a positioned box's is).
pub(super) fn is_z_indexed_item(dom: &Dom<TuiExt>, parent: NodeId, c: &ComputedStyle) -> bool {
    !matches!(c.z_index, ZIndex::Auto) && c.display != Display::Contents && is_item_of(dom, parent)
}

/// The clip `id`'s content paints into, given the clip `id` itself
/// paints into (CSS Overflow 3 §3): a scroll container's padding box; on
/// each `overflow: clip` axis the overflow clip edge — the
/// `overflow-clip-margin` box outset by its margin (§3.2) — and on a
/// `visible` axis beside it no edge at all; a line-clamp container's
/// rows end at its clamp point; `clip` unchanged when nothing clips.
/// Paint and hit-testing share it.
pub(crate) fn children_clip(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle, clip: Rect) -> Rect {
    let Some(ext) = dom.node(id).ext() else {
        return clip;
    };
    // A line-clamp container hides what follows its clamp point (CSS
    // Overflow 4 §4.4), whatever its `overflow`.
    let edges = crate::render::layout_pass::ClipEdges::of_element(dom, id, ext, c);
    if edges == crate::render::layout_pass::ClipEdges::NONE {
        return clip;
    }
    // An edge narrows the clip; an axis that does not clip keeps it.
    let span = |edge: Option<(i32, i32)>, start: u16, len: u16| {
        let (lo, hi) = (i32::from(start), i32::from(start) + i32::from(len));
        let (s, e) = edge.map_or((lo, hi), |(s, e)| (s.max(lo), e.min(hi)));
        (s, (e - s).clamp(0, i32::from(u16::MAX)) as u16)
    };
    let (x, width) = span(edges.x, clip.x, clip.width);
    let (y, height) = span(edges.y, clip.y, clip.height);
    let edge = crate::layout::LayoutRect::new(x, y, width, height);
    layout_rect_to_grid(edge, clip).unwrap_or_else(|| Rect::new(clip.x, clip.y, 0, 0))
}

mod collect;
mod unit;

#[cfg(test)]
thread_local! {
    /// Child nodes the paint walks looked at — the layer collection, the
    /// paint units' own phases and the content recursion (tests only: the
    /// paint cost pin).
    pub(crate) static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Count one child node a paint walk looks at (tests only).
pub(crate) fn visit() {
    #[cfg(test)]
    VISITS.with(|c| c.set(c.get() + 1));
}

pub(crate) use collect::collect_layers;
pub(crate) use unit::{UnitFloat, for_each_unit_box};

#[cfg(test)]
mod tests;
