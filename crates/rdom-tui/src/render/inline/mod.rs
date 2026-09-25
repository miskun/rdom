//! Inline formatting context layout — greedy line packing.
//!
//! Given an IFC block and its content width, produces an
//! [`InlineLayout`]: a list of [`LineBox`]es, each a list of
//! [`InlineFragment`]s.
//!
//! ## Algorithm
//!
//! 1. Walk the block's subtree in document order (see [`walk_subtree`]),
//!    producing a stream of (owner-element, source-text-node,
//!    byte-offset, grapheme) tuples. Owner is the direct element
//!    parent of the text node — for hit-test routing we need to know
//!    which `<code>` / `<b>` / `<p>` a click lands in.
//! 2. Normalize whitespace per the block's cascaded `white_space`
//!    (see [`packer`]). `Normal` / `NoWrap` collapse runs to a single
//!    space and trim IFC edges; `Pre` passes through verbatim.
//! 3. Accumulate visible graphemes into a *pending word* — a run
//!    bracketed by break opportunities (whitespace, CJK boundaries,
//!    hyphen-after).
//! 4. On each break opportunity, attempt to commit the pending word.
//!    If it doesn't fit at the current cursor + pending space, wrap
//!    to a new line.
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

mod caret;
pub(crate) mod generated;
mod packer;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{StyleSlot, TuiExt};
use crate::layout::WhiteSpace;

pub use caret::cell_of_position;
pub(crate) use caret::cells_before_byte;
use packer::LinePacker;

/// One visible chunk of text painted contiguously on a single line
/// with a single owner element + source text node. An inline
/// element whose text wraps produces multiple fragments (one per
/// line). A whitespace-collapsed separator ("a <b>bold</b>") is
/// also a single fragment whose text is `" "`.
///
/// **Atomic inline-block fragments** (`atomic = true`) carry a
/// `Display::InlineBlock` element participating in IFC. Their
/// `text` is empty; their `width` is the box's intrinsic main-
/// axis size including UA pseudo content (`<button>`'s `[ … ]`).
/// Paint renders them via the regular inline-content path at
/// `(x, line_y, width)`; selection skips them; hit-test routes to
/// `node`. Closes the bracketed-button-inside-`<p>` case of
/// `IFC-MIXED-TEXT-INLINEBLOCK-1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineFragment {
    /// The direct element parent of the source text, or — for
    /// `atomic = true` fragments — the inline-block element itself.
    /// Click / hover routes here.
    pub node: NodeId,
    /// The source `Text` node whose data this fragment renders. For
    /// whitespace-collapsed separators, this is the text node that
    /// contained the first collapsed whitespace byte. For
    /// `atomic = true` fragments, set to the inline-block element
    /// (sentinel — there's no source text node).
    pub text_node: NodeId,
    /// Byte offset in `text_node`'s data where this fragment's
    /// first grapheme sits. The runtime's `position_at` walks
    /// fragment graphemes from `x` to compute the hit position.
    /// `0` for atomic fragments.
    pub source_byte_offset: usize,
    /// X offset from the IFC block's content area left edge.
    pub x: u16,
    /// Visible cell width of `text` (or, for atomic fragments,
    /// the inline-block's intrinsic main-axis content size).
    pub width: u16,
    /// Normalized text to paint. No control characters; no leading /
    /// trailing whitespace when this fragment brackets a line.
    /// Empty for `atomic = true` fragments.
    pub text: String,
    /// True iff this fragment is an atomic inline-block box
    /// (`Display::InlineBlock` participating in IFC). See the type
    /// doc for the full contract.
    pub atomic: bool,
}

/// A run of a host's static `::before` / `::after` content on one
/// line. CSS 2.1 §12.1: generated content is an inline box, the first
/// / last child of its host, so the packer lays it out with the text —
/// it wraps, and the text after it starts past it.
///
/// Generated content has no DOM node and no DOM position, so it is kept
/// apart from [`LineBox::fragments`]: hit-testing, the caret,
/// selection highlight and copy only ever see text and atoms, and a
/// click on a generated cell clamps to the nearest text position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFragment {
    /// The element whose pseudo-element this is (paint reads its
    /// `computed_before` / `computed_after`).
    pub host: NodeId,
    /// [`StyleSlot::Before`](crate::ext::StyleSlot::Before) or
    /// [`StyleSlot::After`](crate::ext::StyleSlot::After).
    pub slot: crate::ext::StyleSlot,
    /// X offset from the inline flow's content-area left edge.
    pub x: u16,
    /// Visible cell width of `text`.
    pub width: u16,
    /// The normalized generated text on this line.
    pub text: String,
}

/// One line of inline content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineBox {
    /// Fragments in left-to-right order, each non-overlapping.
    pub fragments: Vec<InlineFragment>,
    /// Generated-content runs on this line, left to right. They occupy
    /// cells between / around `fragments` — never overlapping them.
    pub generated: Vec<GeneratedFragment>,
    /// Total visible width of this line, generated content included
    /// (≤ content width unless a single word overflowed).
    pub width: u16,
}

/// Full inline layout for an IFC block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLayout {
    pub lines: Vec<LineBox>,
    /// The content width this layout was packed for. Paint reuses it
    /// to know where to clip overflowing fragments.
    pub content_width: u16,
}

impl InlineLayout {
    /// Height in lines. Each line occupies one row in the TUI.
    pub fn height(&self) -> u16 {
        self.lines.len() as u16
    }
}

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
            && let Some(index) = dom.node(id).child_nodes().position(|c| c.id() == child)
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
    use crate::node::TuiNodeExt;
    let mut content = dom.node(block).content_layout_rect()?;
    let ext = dom.node(block).ext()?;
    content.x -= ext.scroll_x as i32;
    content.y -= ext.scroll_y as i32;
    Some(content)
}

/// The atomic (`display: inline-block`) fragments of `layout`, each with
/// the outer rect it occupies when the layout is painted at `origin`
/// (one row per line). Both the block pass (anonymous boxes) and the
/// flex pass (single IFC) recurse `layout_node` into these so the
/// inline-block's own subtree lays out; the snapshot exists because the
/// caller cannot hold `&InlineLayout` while mutating the arena.
pub fn atomic_placements(
    layout: &InlineLayout,
    origin: crate::layout::LayoutRect,
) -> Vec<(NodeId, crate::layout::LayoutRect)> {
    let mut atoms = Vec::new();
    for (line_idx, line) in layout.lines.iter().enumerate() {
        let line_y = origin.y + line_idx as i32;
        for fragment in line.fragments.iter().filter(|f| f.atomic) {
            atoms.push((
                fragment.node,
                crate::layout::LayoutRect::new(
                    origin.x + fragment.x as i32,
                    line_y,
                    fragment.width,
                    1,
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
    push_pseudo(dom, block, StyleSlot::Before, &mut packer);
    walk_subtree(dom, block, &mut packer);
    push_pseudo(dom, block, StyleSlot::After, &mut packer);
    packer.finish();
    InlineLayout {
        lines: packer.take_lines(),
        content_width,
    }
}

/// Push `host`'s `slot` pseudo-element if it joins `host`'s own inline
/// content (see [`generated`]). `::before` first pushes the markers of
/// the list items whose first line this is.
fn push_pseudo<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: StyleSlot,
    packer: &mut LinePacker<'a>,
) {
    if slot == StyleSlot::Before {
        for item in generated::deferred_markers(dom, host) {
            if let Some(text) = generated::static_pseudo_text(dom, item, StyleSlot::Before) {
                packer.push_generated(item, StyleSlot::Before, text);
            }
        }
    }
    if let Some(text) = generated::own_inline_pseudo_text(dom, host, slot) {
        packer.push_generated(host, slot, text);
    }
}

/// Pack a **range of direct children** of `parent` as an inline
/// formatting context. Used by `layout_block_children` to populate
/// anonymous block boxes per CSS 2.1 §9.2.1.1 — the block container
/// holds the IFC's whitespace context, but only the listed
/// `direct_children` participate in this anonymous block's content.
///
/// Each entry in `direct_children` must be a NodeId that's a direct
/// child of `parent` (text or element). Text-node children pack as
/// inline runs owned by `parent`; element children pack via
/// `walk_subtree` (same semantics as the full-subtree path).
///
/// `parent`'s static `::before` joins the run that starts at its first
/// in-flow child, its `::after` the run that ends at its last one
/// (CSS 2.1 §9.2.1.1: they are the first / last inline-level content).
pub fn compute_inline_layout_for_run(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[NodeId],
    content_width: u16,
) -> InlineLayout {
    let mut in_flow = dom
        .node(parent)
        .child_nodes()
        .map(|c| c.id())
        .filter(|&c| crate::render::layout_pass::is_in_flow(dom, c));
    let first = in_flow.next();
    let last = in_flow.last().or(first);
    let pseudos = RunPseudos {
        before: first.is_some() && direct_children.first().copied() == first,
        after: last.is_some() && direct_children.last().copied() == last,
    };
    pack_run(dom, parent, direct_children, pseudos, content_width)
}

/// Which of the run's host pseudo-elements a [`pack_run`] includes.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunPseudos {
    pub(crate) before: bool,
    pub(crate) after: bool,
}

/// Pack `direct_children` of `parent` as one inline formatting context,
/// with `parent`'s `::before` / `::after` first / last as `pseudos`
/// asks. An empty `direct_children` packs the pseudo-elements alone.
pub(crate) fn pack_run(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[NodeId],
    pseudos: RunPseudos,
    content_width: u16,
) -> InlineLayout {
    let ws = dom
        .node(parent)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.white_space)
        .unwrap_or(WhiteSpace::Normal);

    use crate::layout::Display;
    let mut packer = LinePacker::new(content_width, ws);
    if pseudos.before {
        push_pseudo(dom, parent, StyleSlot::Before, &mut packer);
    }
    for &child_id in direct_children {
        let child = dom.node(child_id);
        match child.node_type() {
            NodeType::Text => {
                if let Some(data) = child.node_value() {
                    packer.push_text(parent, child_id, data);
                }
            }
            NodeType::Element => {
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child_id);
                    continue;
                }
                // Inline-block participates as an atomic box — see
                // `walk_subtree` for the rationale.
                let display = child
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .map(|c| c.display)
                    .unwrap_or(Display::Block);
                if matches!(display, Display::InlineBlock) {
                    let intrinsic =
                        atomic_inline_block_intrinsic_width(dom, child_id, packer.content_width());
                    packer.push_atomic_inline_block(child_id, intrinsic);
                    continue;
                }
                walk_inline_box(dom, child_id, &mut packer);
            }
            _ => {}
        }
    }
    if pseudos.after {
        push_pseudo(dom, parent, StyleSlot::After, &mut packer);
    }
    packer.finish();
    InlineLayout {
        lines: packer.take_lines(),
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
                // CSS 2.1 §10.8: a `Display::InlineBlock` element
                // participates in IFC as a single atomic inline-
                // level box. Don't recurse into it — the packer
                // emits a width-`intrinsic` placeholder fragment,
                // and paint renders the box's content (including
                // UA pseudos like `<button>`'s `[ ]`) via the
                // regular inline-content path at that rect.
                if matches!(display, Display::InlineBlock) {
                    let intrinsic = atomic_inline_block_intrinsic_width(
                        dom,
                        child.id(),
                        packer.content_width(),
                    );
                    packer.push_atomic_inline_block(child.id(), intrinsic);
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
        packer.push_generated(id, StyleSlot::Before, text);
    }
    walk_subtree(dom, id, packer);
    if let Some(text) = generated::static_pseudo_text(dom, id, StyleSlot::After) {
        packer.push_generated(id, StyleSlot::After, text);
    }
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
