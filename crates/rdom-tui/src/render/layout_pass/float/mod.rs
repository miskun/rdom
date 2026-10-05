//! Floats (CSS 2.1 §9.5): which boxes float, the exclusion area of each
//! block formatting context, placing floats in it, the space it leaves
//! line boxes and blocks, and clearance (§9.5.2).
//!
//! ## Model
//!
//! - [`area::ExclusionArea`] — one block formatting context's floats and
//!   the §9.5.1 placement rules, pure geometry.
//! - A stack of areas, document data for the layout pass ([`enter`] /
//!   [`leave`]): a block container that establishes a block formatting
//!   context (`block::establishes_bfc`) pushes a fresh area while its
//!   children are laid out; a block container that does not uses the
//!   area of the context it is in, so a float in one paragraph shortens
//!   the lines of the next. [`with_area`] lends the current area out
//!   for a placement or a packing — never across a `layout_node`, which
//!   may enter a context of its own.
//! - [`lines::InlineFloats`] — the exclusion-area API the inline packer
//!   consumes ([`lines::LineExclusions`]): the band a line box may use,
//!   where the next float ends, and placing a float met in the inline
//!   content on the current line or the next.
//! - [`size::FloatBox`] — a float's margin box: its used width
//!   (shrink-to-fit, CSS 2.1 §10.3.5) and height, and its margins.
//! - `measure` — the same rules for intrinsic sizes, on a scratch area.
//!
//! A float in a flex or grid container does not float (its children are
//! items: CSS Flexbox §4, CSS Grid 2 §6.1), nor does one of the document
//! root's children — rdom lays those out as the items of its viewport
//! column (DIVERGENCES).

pub(crate) mod area;
mod flow;
pub(crate) mod lines;
pub(in crate::render::layout_pass) mod measure;
pub(crate) mod size;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, FloatSide, LayoutRect, TextDirection};
use crate::style::ComputedStyle;

pub(crate) use area::ExclusionArea;
pub(in crate::render::layout_pass) use flow::{FlowBox, beside_floats};

/// The side `id` floats to (CSS 2.1 §9.5): `None` unless its `float` is
/// not `none` (an absolutely positioned box's computes to `none`, §9.7),
/// it has a box, and its box parent is a block container. The flow-
/// relative keywords resolve against the containing block's `direction`
/// (CSS Logical 1 §2.3).
pub(crate) fn float_side(dom: &Dom<TuiExt>, id: NodeId) -> Option<FloatSide> {
    let c = dom.node(id).ext()?.computed.as_deref()?;
    if c.float == crate::layout::Float::None
        || matches!(c.display, Display::None | Display::Contents)
    {
        return None;
    }
    let parent = crate::render::box_tree::box_parent(dom, id)?;
    let p = dom.node(parent);
    if p.node_type() != NodeType::Element {
        return None;
    }
    let pc = p.ext()?.computed.as_deref()?;
    if !pc.flow.is_block_flow() {
        return None;
    }
    c.float.side(pc.text_direction == TextDirection::Rtl)
}

/// The sides whose floats `id`, styled `c`, clears (CSS 2.1 §9.5.2),
/// `(left, right)`, the flow-relative keywords resolved against its
/// containing block's `direction`.
pub(crate) fn clear_sides(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> (bool, bool) {
    let rtl = crate::render::box_tree::box_parent(dom, id).and_then(|p| {
        dom.node(p)
            .ext()?
            .computed
            .as_deref()
            .map(|pc| pc.text_direction)
    }) == Some(TextDirection::Rtl);
    c.clear.sides(rtl)
}

/// The areas of the block formatting contexts being laid out, innermost
/// last (document data for the layout pass).
#[derive(Debug, Default)]
struct FloatStack(Vec<ExclusionArea>);

/// Open a block formatting context: its children's floats go into a
/// fresh area until [`leave`].
pub(in crate::render::layout_pass) fn enter(dom: &mut Dom<TuiExt>) {
    match dom.document_data_mut::<FloatStack>() {
        Some(stack) => stack.0.push(ExclusionArea::default()),
        None => {
            dom.set_document_data(FloatStack(vec![ExclusionArea::default()]));
        }
    }
}

/// Close the innermost block formatting context; its area.
pub(in crate::render::layout_pass) fn leave(dom: &mut Dom<TuiExt>) -> ExclusionArea {
    let area = dom
        .document_data_mut::<FloatStack>()
        .and_then(|s| s.0.pop());
    debug_assert!(area.is_some(), "float::leave without enter");
    area.unwrap_or_default()
}

/// How many floats the innermost context holds: a [`rewind`] mark taken
/// before a box lays out its children.
pub(in crate::render::layout_pass) fn mark(dom: &Dom<TuiExt>) -> usize {
    dom.document_data::<FloatStack>()
        .and_then(|s| s.0.last())
        .map_or(0, ExclusionArea::len)
}

/// Forget the floats the innermost context took since `mark`: a box that
/// lays its children out again (its scrollbar gutter settled, its scroll
/// offset clamped) places their floats again.
pub(in crate::render::layout_pass) fn rewind(dom: &mut Dom<TuiExt>, mark: usize) {
    if let Some(top) = dom
        .document_data_mut::<FloatStack>()
        .and_then(|s| s.0.last_mut())
    {
        top.truncate(mark);
    }
}

/// Whether a block formatting context is open (its area is on the
/// stack).
pub(in crate::render::layout_pass) fn in_context(dom: &Dom<TuiExt>) -> bool {
    dom.document_data::<FloatStack>()
        .is_some_and(|s| !s.0.is_empty())
}

/// Run `f` with the innermost area lent out, `dom` read-only — a float
/// placement or an inline packing. Outside any context (a box laid out on
/// its own) `f` gets a scratch area.
pub(in crate::render::layout_pass) fn with_area<R>(
    dom: &mut Dom<TuiExt>,
    f: impl FnOnce(&Dom<TuiExt>, &mut ExclusionArea) -> R,
) -> R {
    let mut area = dom
        .document_data_mut::<FloatStack>()
        .and_then(|s| s.0.last_mut())
        .map(std::mem::take);
    let lent = area.is_some();
    let mut scratch = area.take().unwrap_or_default();
    let out = f(dom, &mut scratch);
    if lent
        && let Some(top) = dom
            .document_data_mut::<FloatStack>()
            .and_then(|s| s.0.last_mut())
    {
        *top = scratch;
    }
    out
}

/// Place the float `id` met in block flow — between block-level boxes,
/// where its hypothetical box would have its top at `y` — in the
/// containing block `[x0, x0 + cb_width)` whose content box starts at
/// row `content_top` (CSS 2.1 §9.5.1, its own `clear` too, §9.5.2): its
/// border box.
pub(in crate::render::layout_pass) fn place_in_block_flow(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    at: Placement,
) -> LayoutRect {
    with_area(dom, |dom, area| place(dom, area, id, at))
}

/// Where a float may go: its top not above row `y`, in the containing
/// block `[x0, x0 + cb_width)` whose content box's top is row
/// `content_top` (the edge `margin-trim: block-start` trims at).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) struct Placement {
    pub(in crate::render::layout_pass) y: i32,
    pub(in crate::render::layout_pass) x0: i32,
    pub(in crate::render::layout_pass) cb_width: u16,
    pub(in crate::render::layout_pass) content_top: i32,
}

/// Place float `id` in `area` (and below the floats its `clear` names):
/// its border box.
pub(in crate::render::layout_pass) fn place(
    dom: &Dom<TuiExt>,
    area: &mut ExclusionArea,
    id: NodeId,
    at: Placement,
) -> LayoutRect {
    let fb = size::FloatBox::of(dom, id, at.cb_width);
    place_box(dom, area, id, &fb, at)
}

/// [`place`] with the float's box already measured. CSS Box 4 §3: the
/// containing block's `margin-trim` drops the float's inline-start
/// (inline-end) margin when its margin box would abut that content edge,
/// and its block-start margin when its top is at the block-start one.
pub(in crate::render::layout_pass) fn place_box(
    dom: &Dom<TuiExt>,
    area: &mut ExclusionArea,
    id: NodeId,
    fb: &size::FloatBox,
    at: Placement,
) -> LayoutRect {
    let side = float_side(dom, id).unwrap_or(FloatSide::Left);
    let y = clearance_floor(dom, area, id, at.y);
    let (x0, x1) = (at.x0, at.x0 + i32::from(at.cb_width));
    let trim = trim_of(dom, id);
    let mut fb = *fb;
    if trim.top && y == at.content_top {
        fb.margin_top = 0;
    }
    let position =
        |fb: &size::FloatBox| area.position(side, fb.outer_width(), fb.outer_height(), y, x0, x1);
    let trimmed = match side {
        FloatSide::Left if trim.left => Some(size::FloatBox {
            margin_left: 0,
            ..fb
        }),
        FloatSide::Right if trim.right => Some(size::FloatBox {
            margin_right: 0,
            ..fb
        }),
        _ => None,
    };
    let flush = trimmed
        .map(|t| (t, position(&t)))
        .filter(|(_, m)| match side {
            FloatSide::Left => m.left == x0,
            FloatSide::Right => m.right == x1,
        });
    let (fb, m) = flush.unwrap_or_else(|| (fb, position(&fb)));
    area.push(m);
    fb.border_box(m.left, m.top)
}

/// The edges of `id`'s containing block whose adjoining float margins
/// `margin-trim` drops (CSS Box 4 §3): block-start, and inline-start /
/// -end mapped to left and right by its `direction`.
fn trim_of(dom: &Dom<TuiExt>, id: NodeId) -> crate::layout::Sides<bool> {
    let Some(c) = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| dom.node(p).ext()?.computed.clone())
    else {
        return crate::layout::Sides::new(false, false, false, false);
    };
    let t = c.margin_trim;
    let (left, right) = if c.text_direction == TextDirection::Rtl {
        (t.inline_end, t.inline_start)
    } else {
        (t.inline_start, t.inline_end)
    };
    crate::layout::Sides::new(t.block_start, right, false, left)
}

/// `y`, or the bottom of the floats `id`'s `clear` names when lower
/// (CSS 2.1 §9.5.2; for a float, its top outer edge goes below them).
pub(in crate::render::layout_pass) fn clearance_floor(
    dom: &Dom<TuiExt>,
    area: &ExclusionArea,
    id: NodeId,
    y: i32,
) -> i32 {
    let Some(c) = dom.node(id).ext().and_then(|e| e.computed.as_deref()) else {
        return y;
    };
    let (left, right) = clear_sides(dom, id, c);
    area.clearance(left, right).map_or(y, |b| b.max(y))
}

/// After float `id`, placed at the border box `placed`, was laid out:
/// make its exclusion as tall as the box it got (an automatic height
/// resolves from the laid-out content) — it is the area's last float,
/// as a float's own layout places nothing in this area (it is a block
/// formatting context root).
pub(in crate::render::layout_pass) fn settle_height(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    placed: LayoutRect,
) {
    let Some(got) = dom.node(id).ext().map(|e| e.layout.height) else {
        return;
    };
    let rows = i32::from(got) - i32::from(placed.height);
    if rows != 0 {
        with_area(dom, |_, area| area.grow_last(rows));
    }
}
