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

mod packer;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::WhiteSpace;

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

/// One line of inline content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineBox {
    /// Fragments in left-to-right order, each non-overlapping.
    pub fragments: Vec<InlineFragment>,
    /// Total visible width of this line (≤ content width unless a
    /// single word overflowed).
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
    let mut cur = Some(text_node);
    while let Some(id) = cur {
        if has_inline_layout(dom, id) {
            return Some(InlineFlow::Ifc { block: id });
        }
        if let Some(ext) = dom.node(id).ext() {
            // Find the anon box whose IFC contains a fragment owned
            // by `text_node`. Linear scan — anon-box Vecs are short.
            for (i, anon) in ext.anonymous_blocks.iter().enumerate() {
                for line in &anon.inline_layout.lines {
                    for frag in &line.fragments {
                        if frag.text_node == text_node {
                            return Some(InlineFlow::Anonymous {
                                container: id,
                                index: i,
                            });
                        }
                    }
                }
            }
        }
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
    use crate::node::TuiNodeExt;
    match flow {
        InlineFlow::Ifc { block } => {
            let layout = dom.node(block).ext()?.inline_layout.as_ref()?;
            let content = dom.node(block).content_layout_rect()?;
            Some((layout, content))
        }
        InlineFlow::Anonymous { container, index } => {
            let anon = dom.node(container).ext()?.anonymous_blocks.get(index)?;
            Some((&anon.inline_layout, anon.rect))
        }
    }
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
    walk_subtree(dom, block, &mut packer);
    packer.finish();
    InlineLayout {
        lines: packer.take_lines(),
        content_width,
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
pub fn compute_inline_layout_for_run(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[NodeId],
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
                    let intrinsic = atomic_inline_block_intrinsic_width(dom, child_id);
                    packer.push_atomic_inline_block(child_id, intrinsic);
                    continue;
                }
                walk_subtree(dom, child_id, &mut packer);
            }
            _ => {}
        }
    }
    packer.finish();
    InlineLayout {
        lines: packer.take_lines(),
        content_width,
    }
}

/// Recursively walk `id`'s descendants in document order, feeding
/// every text node's graphemes to `packer`. Descends into
/// `display: inline` elements; `<br>` emits a hard line break.
///
/// Non-element children (comments, fragments) are passed through
/// their descendant element walk.
fn walk_subtree(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker) {
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
                let display = child
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .map(|c| c.display)
                    .unwrap_or(Display::Block);
                if matches!(display, Display::InlineBlock) {
                    let intrinsic = atomic_inline_block_intrinsic_width(dom, child.id());
                    packer.push_atomic_inline_block(child.id(), intrinsic);
                    continue;
                }
                walk_subtree(dom, child.id(), packer);
            }
            _ => {}
        }
    }
}

/// Intrinsic main-axis (row) content width of an inline-block
/// element treated as an atomic IFC box. Includes UA pseudo
/// content (`::before` + `::after`) plus own text/inline content
/// plus padding/border via the existing intrinsic measurement.
fn atomic_inline_block_intrinsic_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    // `intrinsic_size` already factors in pseudo widths +
    // padding + border for Display::InlineBlock — that's the same
    // measurement the flex layout uses to size inline-block flex
    // items. Pass `cross_budget = 0` since IFC packers don't
    // affect inline-block height; only the width matters here.
    crate::render::layout_pass::intrinsic::intrinsic_size(dom, id, crate::layout::Direction::Row, 0)
}
