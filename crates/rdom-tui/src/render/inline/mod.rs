//! Inline formatting context layout — greedy line packing.
//!
//! Given an IFC block and its content width, produces an
//! [`InlineLayout`]: a list of [`LineBox`]es, each a list of
//! [`InlineFragment`]s.
//!
//! ## Algorithm
//!
//! 1. Walk the block's subtree in document order (see `walk_subtree`),
//!    producing a stream of (owner-element, source-text-node,
//!    byte-offset, grapheme) tuples. Owner is the direct element
//!    parent of the text node — for hit-test routing we need to know
//!    which `<code>` / `<b>` / `<p>` a click lands in.
//! 2. Process white space per each text's own `white-space-collapse`
//!    (`white_space`, `run_style`): collapsible runs become one space
//!    (none at a line edge), preserved ones pass through, preserved
//!    segment breaks force a line break.
//! 3. Accumulate visible graphemes into a *pending word* — a run
//!    bracketed by soft wrap opportunities (white space, and the UAX #14
//!    subset of `breaking`: ideographs, hyphens, soft hyphens,
//!    zero-width spaces, under `word-break` / `line-break` / `hyphens`).
//! 4. On each break opportunity, attempt to commit the pending word.
//!    If it doesn't fit at the current cursor + pending space, wrap
//!    to a new line.
//!
//! 5. When a line is done, settle its height (`vertical`): one row,
//!    or as many as its tallest atomic inline block needs, its text on
//!    the baseline row (CSS 2.1 §10.8). The data model is `boxes`.
//!
//! Words longer than the content width overflow their line — CSS's
//! default `overflow-wrap: normal` behavior; `anywhere` / `break-word`
//! break them between graphemes. Where a fragment's painted text is not
//! its source (a soft hyphen shown as `-`), `source_map` maps the two.
//!
//! ## Source tracking for selection
//!
//! Each [`InlineFragment`] records the source text node + byte offset
//! it derives from. This is what drag-selection uses to map a screen
//! click back into a [`rdom_core::Position`]: given a cell (x, y)
//! inside a fragment, walk the fragment's graphemes counting cells
//! until reaching x, then take the cumulative byte length and add
//! to `source_byte_offset`.
//!
//! ## Scope
//!
//! Line breaking is a UAX #14 subset without dictionaries (`breaking`,
//! DIVERGENCES §2): no complex-script word breaking, no automatic
//! hyphenation.

mod align;
mod boxes;
mod breaking;
mod caret;
mod feed;
pub(crate) mod generated;
mod indent;
mod measure;
mod packer;
mod run_style;
mod source_map;
mod transform;
pub(crate) mod vertical;
mod white_space;
mod wrap;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

use crate::render::layout_pass::float::lines::LineExclusions;
pub(crate) use boxes::GeneratedAtom;
pub use boxes::{GeneratedFragment, InlineFragment, InlineLayout, LineBox};
pub(crate) use caret::caret_cell;
pub use caret::cell_of_position;
use feed::{fill_block, fill_run};
pub(crate) use measure::{widest_line, widest_run_line};
use packer::LinePacker;
pub(crate) use white_space::is_collapsible_white_space;

/// True iff `id` has a populated `inline_layout` on its `TuiExt`.
/// Singular variant of [`inline_flow_container`].
pub fn has_inline_layout(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .ext()
        .and_then(|e| e.inline_layout.as_ref())
        .is_some()
}

/// Walk up from `node_id` to the nearest element with a populated
/// `inline_layout`. Inclusive — if `node_id` itself is such an
/// element, returns it.
///
/// This is the "find the inline-flow container that owns this text"
/// lookup. Two kinds of elements have an `inline_layout`:
///
/// 1. **IFC blocks** — elements with `display: inline` children.
///    Their `inline_layout` packs the inline children plus any
///    interleaved text.
/// 2. **Pure-text leaf blocks** — elements with only direct text
///    content and no element children (e.g. `<input>`, `<textarea>`,
///    a `<p>only text</p>`). Their `inline_layout` packs the text
///    against the element's content width.
///
/// **Does NOT find anonymous block boxes** (BFC-1 phase 3) — their
/// inline_layouts live in the parent container's `anonymous_blocks`
/// Vec, not in `inline_layout`. Callers that need anon-box support
/// use [`inline_flow_for_text`] instead.
///
/// Used by caret positioning, mouse hit-test routing, drag-selection
/// anchoring, multi-click word/line expansion, and the caret paint
/// primitive — every path that needs to map a text node back to the
/// inline-flow container that laid it out.
pub fn inline_flow_container(dom: &Dom<TuiExt>, node_id: NodeId) -> Option<NodeId> {
    let mut cur = Some(node_id);
    while let Some(id) = cur {
        if has_inline_layout(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// The inline-flow container holding a given text node — either a
/// classic IFC block (singular `inline_layout`) or an anonymous
/// block box synthesized by the block layout pass (BFC-1 phase 3).
///
/// Returned by [`inline_flow_for_text`] so callers can read the
/// `InlineLayout` and the IFC's content rect without caring which
/// variety they're in. The variants compare by identity — two text
/// nodes in different anon boxes of the same container are NOT in
/// the same flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineFlow {
    /// Singular IFC — `block` owns `inline_layout`. Content rect is
    /// `block`'s `content_layout`.
    Ifc { block: NodeId },
    /// Anonymous block box — slot `index` in `container`'s
    /// `anonymous_blocks` Vec.
    Anonymous { container: NodeId, index: usize },
}

impl InlineFlow {
    /// Owner NodeId for identity comparisons / back-compat. For
    /// anon boxes this is the container — two `Anonymous` flows
    /// on the same container share the owner but have different
    /// indices, so use full equality for identity.
    pub fn owner(&self) -> NodeId {
        match self {
            Self::Ifc { block } => *block,
            Self::Anonymous { container, .. } => *container,
        }
    }
}

/// Resolve `text_node` to its containing [`InlineFlow`]. Walks up
/// from the text node looking for either a singular IFC ancestor
/// or an ancestor with an anonymous block box wrapping the node.
///
/// For most consumers this replaces the
/// [`inline_flow_container`] + manual `ext.inline_layout` lookup
/// pair — see the deprecation note on `inline_flow_container`.
pub fn inline_flow_for_text(dom: &Dom<TuiExt>, text_node: NodeId) -> Option<InlineFlow> {
    // Walk up from the text node. At an ancestor with anonymous
    // boxes, the box wrapping the text is the one whose `child_range`
    // covers the direct child the text sits under — no fragment scan.
    let mut child = text_node;
    let mut cur = dom.node(text_node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if has_inline_layout(dom, id) {
            return Some(InlineFlow::Ifc { block: id });
        }
        if let Some(ext) = dom.node(id).ext()
            && !ext.anonymous_blocks.is_empty()
            && let Some(index) = box_index(dom, id, text_node, child)
            && let Some(i) = ext
                .anonymous_blocks
                .iter()
                .position(|anon| anon.child_range.0 <= index && index < anon.child_range.1)
        {
            return Some(InlineFlow::Anonymous {
                container: id,
                index: i,
            });
        }
        child = id;
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// The index in `container`'s box sequence (`box_tree::box_sequence`)
/// of the item holding `node`, `child` being `node`'s ancestor-or-self
/// among `container`'s child nodes. Without a box-less child the
/// sequence is the child nodes, so the index is `child`'s; with one — or
/// for a flex or grid container (`box_tree::item_sequence`) — it is
/// built once per lookup, in one walk.
fn box_index(dom: &Dom<TuiExt>, container: NodeId, node: NodeId, child: NodeId) -> Option<usize> {
    use crate::render::box_tree::{
        box_sequence, is_contents, is_flex_or_grid_container, item_sequence,
    };
    // A flex or grid container's anonymous items index its item
    // sequence, which holds its pseudo-elements and its box-less
    // children's contents.
    let items_of = is_flex_or_grid_container(dom, container);
    // With no box-less child and no `::before` in the sequence (a
    // block-level or floated one) the sequence is the child nodes.
    if !items_of
        && !crate::render::inline::generated::sequence_pseudos(dom, container).before
        && !dom
            .node(container)
            .child_nodes()
            .any(|c| is_contents(dom, c.id()))
    {
        return dom
            .node(container)
            .child_nodes()
            .position(|c| c.id() == child);
    }
    // The item is `node` or its nearest ancestor in the sequence.
    let items = if items_of {
        item_sequence(dom, container)
    } else {
        box_sequence(dom, container)
    };
    let mut cur = Some(node);
    while let Some(n) = cur {
        if let Some(i) = items.iter().position(|&it| it == BoxItem::Node(n)) {
            return Some(i);
        }
        if n == child {
            return None;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Look up the `InlineLayout` for an [`InlineFlow`]. Returns
/// `(layout, content_rect)` — both needed by caret arithmetic and
/// hit-test fragment lookup.
pub fn inline_flow_layout(
    dom: &Dom<TuiExt>,
    flow: InlineFlow,
) -> Option<(&InlineLayout, crate::layout::LayoutRect)> {
    match flow {
        InlineFlow::Ifc { block } => {
            let layout = dom.node(block).ext()?.inline_layout.as_ref()?;
            let content = scrolled_content_rect(dom, block)?;
            Some((layout, content))
        }
        InlineFlow::Anonymous { container, index } => {
            let anon = dom.node(container).ext()?.anonymous_blocks.get(index)?;
            Some((&anon.inline_layout, anon.rect))
        }
    }
}

/// The IFC block's content rect in **viewport** coordinates: the layout
/// pass stores `content_layout` unscrolled, and an inline flow's lines
/// are packed from its top, so a block that is itself a scroll
/// container (a `<textarea>` taller than its box) has its first
/// `scroll_y` lines above the scrollport. Every consumer that maps
/// `line_index ↔ row` — paint, hit-test, caret placement, keyboard
/// movement — goes through this so they agree.
pub fn scrolled_content_rect(
    dom: &Dom<TuiExt>,
    block: NodeId,
) -> Option<crate::layout::LayoutRect> {
    let mut content = dom.node(block).content_layout_rect()?;
    let ext = dom.node(block).ext()?;
    content.x -= ext.scroll_x;
    content.y -= ext.scroll_y;
    Some(content)
}

/// The atomic (`display: inline-block`) fragments of `layout`, each with
/// the border-box rect it occupies when the layout is painted at
/// `origin` — placed in its line box on the line's baseline
/// (`vertical`). Both the block pass (anonymous boxes) and the
/// flex pass (single IFC) recurse `layout_node` into these so the
/// inline-block's own subtree lays out; the snapshot exists because the
/// caller cannot hold `&InlineLayout` while mutating the arena.
pub fn atomic_placements(
    layout: &InlineLayout,
    origin: crate::layout::LayoutRect,
) -> Vec<(NodeId, crate::layout::LayoutRect)> {
    let mut atoms = Vec::new();
    for line in &layout.lines {
        for fragment in line.fragments.iter().filter(|f| f.atomic) {
            atoms.push((
                fragment.node,
                crate::layout::LayoutRect::new(
                    origin.x + fragment.x,
                    origin.y + i32::from(line.top) + i32::from(fragment.y),
                    fragment.width,
                    fragment.height,
                ),
            ));
        }
    }
    atoms
}

/// Entry point: compute the inline layout for `block` at
/// `content_width`. Idempotent — calling twice with the same inputs
/// yields identical output.
pub fn compute_inline_layout(dom: &Dom<TuiExt>, block: NodeId, content_width: u16) -> InlineLayout {
    compute_inline_layout_around(dom, block, content_width, None)
}

/// [`compute_inline_layout`], its line boxes shortened by the floats
/// `exclusions` describes and its own floats placed there (CSS 2.1
/// §9.5).
pub(crate) fn compute_inline_layout_around<'a>(
    dom: &'a Dom<TuiExt>,
    block: NodeId,
    content_width: u16,
    exclusions: Option<&'a mut dyn LineExclusions>,
) -> InlineLayout {
    let packer = packer_for(dom, block, content_width, true, exclusions);
    let lines = wrap::pack(packer, wrap_style(dom, block), |p| {
        fill_block(dom, block, p)
    });
    InlineLayout {
        lines,
        content_width,
    }
}

/// A packer for the inline formatting context of the block container
/// `block`: its lines starting at its inline-start edge
/// — the right one under `direction: rtl` (CSS Writing Modes 4 §2.1) —
/// indented by its `text-indent` (CSS Text 3 §8.1; `first_formatted` when
/// the flow's first line is the block's first formatted line), beside
/// the floats `exclusions` describes.
fn packer_for<'a>(
    dom: &'a Dom<TuiExt>,
    block: NodeId,
    content_width: u16,
    first_formatted: bool,
    exclusions: Option<&'a mut dyn LineExclusions>,
) -> LinePacker<'a> {
    let computed = dom.node(block).ext().and_then(|e| e.computed.as_ref());
    let rtl = computed.is_some_and(|c| c.text_direction == crate::layout::TextDirection::Rtl);
    let indent = computed.map_or_else(indent::LineIndent::default, |c| {
        indent::LineIndent::of(&c.text.text_indent, content_width, first_formatted)
    });
    let align = computed.map_or_else(align::TextAlignment::default, |c| {
        align::TextAlignment::of(&c.text)
    });
    let packer = LinePacker::new(content_width)
        .starting_right(rtl)
        .indented(indent)
        .aligned(align);
    match exclusions {
        Some(ex) => packer.around(ex),
        None => packer,
    }
}

/// Pack a **range of direct children** of `parent` as an inline
/// formatting context, as the block pass populates an anonymous block
/// box per CSS 2.1 §9.2.1.1 (it calls the crate-internal `pack_run`
/// with the pseudos it already knows) — the block container
/// holds the IFC's whitespace context, but only the listed
/// `direct_children` participate in this anonymous block's content.
///
/// Each entry in `direct_children` must be a NodeId that's a direct
/// child of `parent` (text or element). Text-node children pack as
/// inline runs owned by `parent`; element children pack via
/// `walk_subtree` (same semantics as the full-subtree path).
///
/// `parent`'s static `::before` joins the run that holds its first
/// line-bearing child, its `::after` the run that holds its last one
/// (CSS 2.1 §9.2.1.1: they are the first / last inline-level content;
/// collapsible whitespace-only text and comments bear no line) — the
/// placement the block pass gives the same run
/// (`generated::run_pseudos`).
pub fn compute_inline_layout_for_run(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[NodeId],
    content_width: u16,
) -> InlineLayout {
    let items: Vec<BoxItem> = direct_children.iter().map(|&c| BoxItem::Node(c)).collect();
    let pseudos = generated::run_pseudos(dom, parent, &items);
    pack_run(dom, parent, &items, pseudos, content_width, None)
}

/// The content of `host`'s `slot` pseudo-element's own box — its
/// generated text — packed `width` cells wide in the box's `style`: its
/// CSS Text values, its lines starting at its inline-start edge.
pub(crate) fn pack_generated(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: crate::ext::PseudoSlot,
    style: &crate::style::ComputedStyle,
    width: u16,
) -> InlineLayout {
    let rtl = style.text_direction == crate::layout::TextDirection::Rtl;
    let indent = indent::LineIndent::of(&style.text.text_indent, width, true);
    let packer = LinePacker::new(width)
        .starting_right(rtl)
        .indented(indent)
        .aligned(align::TextAlignment::of(&style.text));
    let text = generated::static_pseudo_text(dom, host, slot.into());
    let lines = wrap::pack(packer, style.text.text_wrap_style, |p| {
        if let Some(text) = text {
            p.push_generated(host, slot, text, run_style::RunStyle::of(style));
        }
    });
    InlineLayout {
        lines,
        content_width: width,
    }
}

/// Which of the run's host pseudo-elements a [`pack_run`] includes.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunPseudos {
    pub(crate) before: bool,
    pub(crate) after: bool,
}

/// Pack `direct_children` of `parent` — items of its box sequence
/// (`box_tree::box_sequence`) — as one inline formatting context,
/// with `parent`'s `::before` / `::after` first / last as `pseudos`
/// asks. An empty `direct_children` packs the pseudo-elements alone.
/// The lines are shortened by the floats `exclusions` describes, and the
/// run's floats placed there (CSS 2.1 §9.5); without it a float in the
/// run is not packed.
pub(crate) fn pack_run<'a>(
    dom: &'a Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[BoxItem],
    pseudos: RunPseudos,
    content_width: u16,
    exclusions: Option<&'a mut dyn LineExclusions>,
) -> InlineLayout {
    // The run holds the parent's first line-bearing content when it holds
    // its `::before` edge (`generated::run_pseudos`): its first line is the
    // parent's first formatted line (CSS Text 3 §8.1).
    let packer = packer_for(dom, parent, content_width, pseudos.before, exclusions);
    let lines = wrap::pack(packer, wrap_style(dom, parent), |p| {
        fill_run(dom, parent, direct_children, pseudos, p);
    });
    InlineLayout {
        lines,
        content_width,
    }
}

/// The `text-wrap-style` of the block container `block` (CSS Text 4: it
/// applies to block containers).
fn wrap_style(dom: &Dom<TuiExt>, block: NodeId) -> crate::layout::TextWrapStyle {
    dom.node(block)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.text.text_wrap_style)
        .unwrap_or_default()
}
