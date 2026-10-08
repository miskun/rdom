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
//! items: CSS Flexbox §4, CSS Grid 2 §6.1). The document root's children
//! float in the initial containing block's block formatting context
//! (`box_tree::icb`).

pub(crate) mod area;
mod flow;
pub(crate) mod lines;
pub(in crate::render::layout_pass) mod measure;
pub(crate) mod size;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, FloatSide, LayoutRect, TextDirection};
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

pub(crate) use area::ExclusionArea;
pub(in crate::render::layout_pass) use flow::{FlowBox, beside_floats_in};

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
    // The initial containing block is a `ltr` block container
    // (`box_tree::icb`): the document root's children float.
    if crate::render::box_tree::icb::is_icb(dom, parent) {
        return c.float.side(false);
    }
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

/// The side the box `item` floats to: an element's [`float_side`]; a
/// static `::before` / `::after`'s by the same rule (CSS Pseudo 4 §2: it
/// floats as an element would), its box parent its host's box — the host,
/// or past a box-less host the box above it.
pub(crate) fn float_side_of(dom: &Dom<TuiExt>, item: BoxItem) -> Option<FloatSide> {
    let (host, slot) = match item {
        BoxItem::Node(id) => return float_side(dom, id),
        BoxItem::Generated(host, slot) => (host, slot),
    };
    let c = pseudo_style(dom, host, slot)?;
    if c.float == crate::layout::Float::None {
        return None;
    }
    // Only a static pseudo-element with `content` has a box of its flow
    // — or a `::first-letter`, whose text is its letter's (the packer
    // floats it only when there is one).
    if slot != crate::ext::PseudoSlot::FirstLetter {
        crate::render::inline::generated::static_pseudo_text(dom, host, slot.into())?;
    }
    let parent = generated_box_parent(dom, host)?;
    let pc = dom.node(parent).ext()?.computed.as_deref()?;
    if !pc.flow.is_block_flow() {
        return None;
    }
    c.float.side(pc.text_direction == TextDirection::Rtl)
}

/// The computed style of `host`'s `slot` pseudo-element.
fn pseudo_style(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: crate::ext::PseudoSlot,
) -> Option<&ComputedStyle> {
    dom.node(host).ext()?.computed_pseudo(slot).map(|c| &**c)
}

/// The element box a pseudo-element of `host` is laid out in: `host`,
/// or — `host` being box-less (CSS Display 3 §2.5) — its box parent.
fn generated_box_parent(dom: &Dom<TuiExt>, host: NodeId) -> Option<NodeId> {
    let parent = if crate::render::box_tree::is_contents(dom, host) {
        crate::render::box_tree::box_parent(dom, host)?
    } else {
        host
    };
    (dom.node(parent).node_type() == NodeType::Element).then_some(parent)
}

/// Whether the box `item` floats ([`float_side_of`]).
pub(crate) fn is_float_item(dom: &Dom<TuiExt>, item: BoxItem) -> bool {
    float_side_of(dom, item).is_some()
}

/// The style of the box `item`: an element's, or a pseudo-element's.
fn style_of(dom: &Dom<TuiExt>, item: BoxItem) -> Option<&ComputedStyle> {
    match item {
        BoxItem::Node(id) => dom.node(id).ext()?.computed.as_deref(),
        BoxItem::Generated(host, slot) => pseudo_style(dom, host, slot),
    }
}

/// The box `item` is laid out in (its containing block for in-flow and
/// floated content, CSS 2.1 §10.1).
fn box_parent_of(dom: &Dom<TuiExt>, item: BoxItem) -> Option<NodeId> {
    match item {
        BoxItem::Node(id) => crate::render::box_tree::box_parent(dom, id),
        BoxItem::Generated(host, _) => generated_box_parent(dom, host),
    }
}

/// The sides whose floats `id`, styled `c`, clears (CSS 2.1 §9.5.2),
/// `(left, right)`, the flow-relative keywords resolved against its
/// containing block's `direction`.
pub(crate) fn clear_sides(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> (bool, bool) {
    clear_sides_of(dom, BoxItem::Node(id), c)
}

/// [`clear_sides`] for the box `item` — an element or a pseudo-element.
pub(crate) fn clear_sides_of(dom: &Dom<TuiExt>, item: BoxItem, c: &ComputedStyle) -> (bool, bool) {
    let rtl = box_parent_of(dom, item).and_then(|p| {
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
/// lays its children out again places their floats again. The one such
/// box whose children's floats are in an enclosing context's area is a
/// block that is no scroll container dropping a stale scroll offset (CSS
/// Overflow 3 §3.1: it has none) — `float/bfc.rs`'s
/// `laying_a_block_out_again_places_its_floats_once` fails without it; a
/// scroll container settling its gutter is a formatting context root,
/// whose children's floats are in its own area.
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

/// Place the float `item` met in block flow — between block-level boxes,
/// where its hypothetical box would have its top at `y` — in the
/// containing block `[x0, x0 + cb_width)` whose content box starts at
/// row `content_top` (CSS 2.1 §9.5.1, its own `clear` too, §9.5.2): its
/// border box.
pub(in crate::render::layout_pass) fn place_in_block_flow(
    dom: &mut Dom<TuiExt>,
    item: BoxItem,
    at: Placement,
) -> PlacedFloat {
    with_area(dom, |dom, area| place(dom, area, item, at))
}

/// A float placed in its formatting context's area: the box, its border
/// box, and its index in the area — by which [`lay_out`] settles its
/// exclusion to the height layout gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlacedFloat {
    pub(crate) item: BoxItem,
    pub(crate) rect: LayoutRect,
    pub(crate) index: usize,
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

/// Place float `item` in `area` (and below the floats its `clear`
/// names): its border box.
pub(in crate::render::layout_pass) fn place(
    dom: &Dom<TuiExt>,
    area: &mut ExclusionArea,
    item: BoxItem,
    at: Placement,
) -> PlacedFloat {
    let fb = size::FloatBox::of(dom, item, at.cb_width);
    place_box(dom, area, item, &fb, at)
}

/// [`place`] with the float's box already measured. CSS Box 4 §3: the
/// containing block's `margin-trim` drops the float's inline-start
/// (inline-end) margin when its margin box would abut that content edge,
/// and its block-start margin when its top is at the block-start one.
pub(in crate::render::layout_pass) fn place_box(
    dom: &Dom<TuiExt>,
    area: &mut ExclusionArea,
    item: BoxItem,
    fb: &size::FloatBox,
    at: Placement,
) -> PlacedFloat {
    let side = float_side_of(dom, item).unwrap_or(FloatSide::Left);
    let y = clearance_floor(dom, area, item, at.y);
    let (x0, x1) = (at.x0, at.x0 + i32::from(at.cb_width));
    let trim = trim_of(dom, item);
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
    let index = area.push(m);
    PlacedFloat {
        item,
        rect: fb.border_box(m.left, m.top),
        index,
    }
}

/// The edges of `item`'s containing block whose adjoining float margins
/// `margin-trim` drops (CSS Box 4 §3): block-start, and inline-start /
/// -end mapped to left and right by its `direction`.
fn trim_of(dom: &Dom<TuiExt>, item: BoxItem) -> crate::layout::Sides<bool> {
    let Some(c) = box_parent_of(dom, item).and_then(|p| dom.node(p).ext()?.computed.clone()) else {
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

/// `y`, or the bottom of the floats `item`'s `clear` names when lower
/// (CSS 2.1 §9.5.2; for a float, its top outer edge goes below them).
pub(in crate::render::layout_pass) fn clearance_floor(
    dom: &Dom<TuiExt>,
    area: &ExclusionArea,
    item: BoxItem,
    y: i32,
) -> i32 {
    let Some(c) = style_of(dom, item) else {
        return y;
    };
    let (left, right) = clear_sides_of(dom, item, c);
    area.clearance(left, right).map_or(y, |b| b.max(y))
}

/// Lay the float `placed` out in a containing block `cb_width` cells wide,
/// at the border box it was placed at, and settle its exclusion to the
/// height it got ([`settle_height`]): an element by `layout_node`; a
/// `::before` / `::after` as its own box (`items::AnonymousItem::pseudo`),
/// kept among the floated pseudo-elements of `owner` — the box whose
/// children are being laid out — which paint and hit-testing read.
pub(in crate::render::layout_pass) fn lay_out(
    dom: &mut Dom<TuiExt>,
    owner: NodeId,
    placed: PlacedFloat,
    cb_width: u16,
) {
    let (host, slot) = match placed.item {
        BoxItem::Node(id) => {
            crate::render::layout_pass::layout_node(dom, id, placed.rect, cb_width);
            settle_height(dom, id, placed);
            return;
        }
        BoxItem::Generated(host, slot) => (host, slot),
    };
    let Some(pseudo) = crate::render::layout_pass::items::AnonymousItem::pseudo(dom, host, slot)
    else {
        return;
    };
    let laid_out = pseudo.lay_out(dom, placed.rect, cb_width);
    if let Some(ext) = dom.node_mut(owner).ext_mut() {
        ext.floated_pseudos
            .get_or_insert_with(Default::default)
            .push(laid_out);
    }
}

/// After the float element `id`, `placed`, was laid out: make its
/// exclusion as tall as the box it got (an automatic height resolves from
/// the laid-out content, which the measured height it was placed at may
/// miss: DIVERGENCES §2) — by its index, as the area's floats after it
/// were placed by then too when the packer met it. The area is the
/// innermost one again: a float is a block formatting context root, so
/// its own layout places nothing in it. (A pseudo-element's box is laid
/// out at the border box it was placed at.)
fn settle_height(dom: &mut Dom<TuiExt>, id: NodeId, placed: PlacedFloat) {
    let Some(got) = dom.node(id).ext().map(|e| e.layout.height) else {
        return;
    };
    let rows = i32::from(got) - i32::from(placed.rect.height);
    if rows != 0 {
        with_area(dom, |_, area| area.grow(placed.index, rows));
    }
}
