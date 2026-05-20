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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineFragment {
    /// The direct element parent of the source text. Click / hover
    /// routes to this node. For text directly under the IFC block,
    /// this is the IFC block itself.
    pub node: NodeId,
    /// The source `Text` node whose data this fragment renders. For
    /// whitespace-collapsed separators, this is the text node that
    /// contained the first collapsed whitespace byte. Used by
    /// selection to map (x, y) → `Position { node: text_node, offset }`.
    pub text_node: NodeId,
    /// Byte offset in `text_node`'s data where this fragment's
    /// first grapheme sits. The runtime's `position_at` walks
    /// fragment graphemes from `x` to compute the hit position.
    pub source_byte_offset: usize,
    /// X offset from the IFC block's content area left edge.
    pub x: u16,
    /// Visible cell width of `text`.
    pub width: u16,
    /// Normalized text to paint. No control characters; no leading /
    /// trailing whitespace when this fragment brackets a line.
    pub text: String,
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

/// Recursively walk `id`'s descendants in document order, feeding
/// every text node's graphemes to `packer`. Descends into
/// `display: inline` elements; `<br>` emits a hard line break.
///
/// Non-element children (comments, fragments) are passed through
/// their descendant element walk.
fn walk_subtree(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker) {
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
                } else {
                    walk_subtree(dom, child.id(), packer);
                }
            }
            _ => {}
        }
    }
}
