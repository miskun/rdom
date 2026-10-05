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
//! 2. Normalize whitespace per the block's cascaded `white_space`
//!    (see `packer`). `Normal` / `NoWrap` collapse runs to a single
//!    space and trim IFC edges; `Pre` passes through verbatim.
//! 3. Accumulate visible graphemes into a *pending word* — a run
//!    bracketed by break opportunities (whitespace, CJK boundaries,
//!    hyphen-after).
//! 4. On each break opportunity, attempt to commit the pending word.
//!    If it doesn't fit at the current cursor + pending space, wrap
//!    to a new line.
//!
//! 5. When a line is done, settle its height (`vertical`): one row,
//!    or as many as its tallest atomic inline block needs, its text on
//!    the baseline row (CSS 2.1 §10.8). The data model is `boxes`.
//!
//! Words longer than the content width overflow their line — CSS's
//! default `overflow-wrap: normal` behavior. Paint clips.
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
//! Phase D (the original inline work): whitespace + CJK + hyphen-after
//! break opportunities. UAX #14 line breaking (soft hyphen, complex-
//! script clustering) is out of scope.

mod align;
mod boxes;
mod caret;
pub(crate) mod generated;
mod packer;
pub(crate) mod vertical;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, StyleSlot, TuiExt};
use crate::layout::WhiteSpace;
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

pub use boxes::{GeneratedFragment, InlineFragment, InlineLayout, LineBox};
pub use caret::cell_of_position;
pub(crate) use caret::cells_before_byte;
use packer::LinePacker;

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
/// sequence is the child nodes, so the index is `child`'s; with one it
/// is built once per lookup, in one walk (`box_sequence`).
fn box_index(dom: &Dom<TuiExt>, container: NodeId, node: NodeId, child: NodeId) -> Option<usize> {
    use crate::render::box_tree::{box_sequence, is_contents};
    // With no box-less child the sequence is the child nodes.
    if !dom
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
    let items = box_sequence(dom, container);
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
                    origin.x + fragment.x as i32,
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
    let ws = dom
        .node(block)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.white_space)
        .unwrap_or(WhiteSpace::Normal);

    let mut packer = LinePacker::new(content_width, ws);
    push_pseudo(dom, block, PseudoSlot::Before, &mut packer);
    walk_subtree(dom, block, &mut packer);
    push_pseudo(dom, block, PseudoSlot::After, &mut packer);
    packer.finish();
    let mut lines = packer.take_lines();
    align::start_lines_at_inline_start(dom, block, &mut lines, content_width);
    InlineLayout {
        lines,
        content_width,
    }
}

/// Push `host`'s `slot` pseudo-element if it joins `host`'s own inline
/// content (see [`generated`]). `::before` first pushes the markers of
/// the list items whose first line this is.
fn push_pseudo<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    packer: &mut LinePacker<'a>,
) {
    if slot == PseudoSlot::Before {
        for item in generated::deferred_markers(dom, host) {
            if let Some(text) = generated::static_pseudo_text(dom, item, StyleSlot::Before) {
                packer.push_generated(item, PseudoSlot::Before, text);
            }
        }
    }
    if let Some(text) = generated::own_inline_pseudo_text(dom, host, slot.into()) {
        packer.push_generated(host, slot, text);
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
    pack_run(dom, parent, &items, pseudos, content_width)
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
pub(crate) fn pack_run(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[BoxItem],
    pseudos: RunPseudos,
    content_width: u16,
) -> InlineLayout {
    let ws = dom
        .node(parent)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.white_space)
        .unwrap_or(WhiteSpace::Normal);

    let mut packer = LinePacker::new(content_width, ws);
    if pseudos.before {
        push_pseudo(dom, parent, PseudoSlot::Before, &mut packer);
    }
    for &item in direct_children {
        let child_id = match item {
            BoxItem::Node(n) => n,
            // The `::before` / `::after` of a box-less child that holds
            // a block box: an inline box of this flow, hosted by it.
            BoxItem::Generated(host, slot) => {
                if let Some(text) = crate::render::box_tree::generated_text(dom, host, slot) {
                    packer.push_generated(host, slot, text);
                }
                continue;
            }
        };
        let child = dom.node(child_id);
        // A text node directly in a box-less child that holds a block
        // box is owned by that child (its parent), not by `parent`.
        let owner = child.parent_node().map_or(parent, |p| p.id());
        match child.node_type() {
            NodeType::Text => {
                if let Some(data) = child.node_value() {
                    packer.push_text(owner, child_id, data);
                }
            }
            NodeType::Element => {
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child_id);
                    continue;
                }
                // An atomic inline participates as one box — see
                // `walk_subtree` for the rationale.
                if child
                    .computed()
                    .is_some_and(crate::render::box_tree::is_atomic_inline)
                {
                    push_atom(dom, child_id, &mut packer);
                    continue;
                }
                walk_inline_box(dom, child_id, &mut packer);
            }
            _ => {}
        }
    }
    if pseudos.after {
        push_pseudo(dom, parent, PseudoSlot::After, &mut packer);
    }
    packer.finish();
    let mut lines = packer.take_lines();
    align::start_lines_at_inline_start(dom, parent, &mut lines, content_width);
    InlineLayout {
        lines,
        content_width,
    }
}

/// Recursively walk `id`'s descendants in document order, feeding
/// every text node's graphemes to `packer`. Descends into
/// `display: inline` elements (their pseudo-elements included — see
/// [`walk_inline_box`]); `<br>` emits a hard line break.
///
/// Non-element children (comments, fragments) are passed through
/// their descendant element walk.
fn walk_subtree<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    use crate::layout::Display;
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Text => {
                // Owner is `id` — the direct element parent. Text
                // node's id goes in too for source-offset tracking.
                if let Some(data) = child.node_value() {
                    packer.push_text(id, child.id(), data);
                }
            }
            NodeType::Element => {
                use crate::layout::Position;
                let (display, position) = child
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .map(|c| (c.display, c.position))
                    .unwrap_or((Display::Block, Position::Static));
                // Out-of-flow descendants contribute nothing to the
                // inline formatting context: `display: none` generates
                // no box, and `position: absolute|fixed` boxes are
                // placed independently by phase-2 positioning. Skipping
                // them keeps their text out of an ancestor's inline run
                // — e.g. a collapsed tree branch (`[role=group]` set to
                // `display: none`) must not leak "hidden-child" into the
                // parent treeitem's text, and a chip with an absolutely-
                // positioned dropdown must pack only the chip's own text.
                if display == Display::None
                    || matches!(position, Position::Absolute | Position::Fixed)
                {
                    continue;
                }
                // <br> is a hard break. Matches HTML's baked-in
                // behavior; recognized by tag name rather than by a
                // Display variant to avoid complicating the cascade
                // for a one-element special case.
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child.id());
                    continue;
                }
                // CSS 2.1 §10.8: an atomic inline (`inline-block`,
                // `inline-flex`, `box_tree::is_atomic_inline`)
                // participates in IFC as a single atomic inline-
                // level box. Don't recurse into it — the packer
                // emits one fragment of its width and rows, the layout
                // pass lays the element out at that rect and paint
                // paints it there as a box, at its turn in the line.
                if child
                    .computed()
                    .is_some_and(crate::render::box_tree::is_atomic_inline)
                {
                    push_atom(dom, child.id(), packer);
                    continue;
                }
                walk_inline_box(dom, child.id(), packer);
            }
            _ => {}
        }
    }
}

/// Feed one in-flow inline element: its static `::before`, its
/// content, its static `::after`. CSS 2.1 §12.1: the pseudo-elements
/// are the element's first / last inline children, so they pack at its
/// start / end, in its line flow (they wrap, and the text beside them
/// shifts). They land in [`LineBox::generated`], hosted by the element.
fn walk_inline_box<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    if let Some(text) = generated::static_pseudo_text(dom, id, StyleSlot::Before) {
        packer.push_generated(id, PseudoSlot::Before, text);
    }
    walk_subtree(dom, id, packer);
    if let Some(text) = generated::static_pseudo_text(dom, id, StyleSlot::After) {
        packer.push_generated(id, PseudoSlot::After, text);
    }
}

/// Push the inline block `id` as an atom: its width and its rows in
/// the line (`vertical`).
fn push_atom(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'_>) {
    let cb_width = packer.content_width();
    let width = atomic_inline_block_intrinsic_width(dom, id, cb_width);
    let rows = vertical::atom_rows(dom, id, width, cb_width);
    packer.push_atomic_inline_block(id, width, rows);
}

/// Intrinsic main-axis (row) content width of an inline-block
/// element treated as an atomic IFC box. Includes UA pseudo
/// content (`::before` + `::after`) plus own text/inline content
/// plus padding/border via the existing intrinsic measurement.
fn atomic_inline_block_intrinsic_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    containing_block_width: u16,
) -> u16 {
    // `intrinsic_size` already factors in pseudo widths +
    // padding + border for Display::InlineBlock — that's the same
    // measurement the flex layout uses to size inline-block flex
    // items. Pass `cross_budget = 0` since IFC packers don't
    // affect inline-block height; only the width matters here.
    // The atom's containing block is the IFC's block container, whose
    // content width is definite: percent padding / margins resolve
    // against it (CSS 2.1 §8.4).
    crate::render::layout_pass::intrinsic::intrinsic_size(
        dom,
        id,
        crate::layout::Direction::Row,
        0,
        containing_block_width,
    )
}
