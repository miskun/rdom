//! IFC detection — is an element an inline formatting context?
//!
//! An element establishes an **IFC** when at least one of its
//! element children participates as inline-level (Display::Inline
//! OR Display::InlineBlock per CSS 2.1 §9.2.1) AND no children are
//! block-level. Mixed block + inline is a cascade error here — the
//! block-layout pass handles it via anonymous block boxes (CSS
//! 2.1 §9.2.1.1, see `render/layout_pass/block.rs`).
//!
//! **Pure-text blocks (`<note>only text</note>`) are deliberately
//! NOT IFC.** They're routed through the non-IFC paint path
//! (`paint_inline_content`), which handles `::before` / own text /
//! `::after` chrome — the IFC paint path (`paint_ifc`) reads from
//! a pre-baked `InlineLayout` that today does not include pseudo
//! content. Their intrinsic *height* is measured via
//! `compute_inline_layout` (see `intrinsic.rs`) so wrap is
//! respected, but the IFC predicate stays false so paint keeps
//! seeing the static pseudos. Unifying the two paths is deferred
//! until pseudo content is integrated into `compute_inline_layout`.
//!
//! **Display::InlineBlock in IFC** (BFC-1 phase 3.5b): an
//! inline-block child participates in IFC as an atomic inline-
//! level box (CSS 2.1 §10.8) — the IFC packer emits one fragment
//! per inline-block carrying the box's intrinsic width, and paint
//! renders it via the regular `paint_inline_content` path at that
//! rect. UA pseudo content (`<button>`'s `[ ]` brackets) shows
//! through.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Display;

/// True iff `id`'s children establish an inline formatting context.
/// Used by both layout (to skip flex distribution + populate
/// `inline_layout`) and paint (to switch to fragment-driven paint).
pub(crate) fn is_ifc_block(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let mut has_inline = false;
    for child in dom.node(id).child_nodes() {
        if child.node_type() != NodeType::Element {
            continue;
        }
        let display = child
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.display)
            .unwrap_or(Display::Block);
        match display {
            // `Inline` triggers IFC: its text packs into the
            // parent's inline flow.
            Display::Inline => has_inline = true,
            // `InlineBlock` neither triggers nor disqualifies. When
            // it appears alongside an `Inline` sibling (mixed text +
            // inline + inline-block), the IFC packer treats it
            // atomically (BFC-1 phase 3.5b). When it appears alone
            // or only with text, the parent stays a flex container
            // (the inline-block is a flex item with intrinsic
            // sizing).
            Display::InlineBlock => continue,
            // Display::None children are invisible and don't
            // participate in layout — they don't count as inline
            // but also don't disqualify an IFC (treat like a
            // whitespace/comment child).
            Display::None => continue,
            // Block-level child → not IFC. The block-layout pass
            // will partition into anonymous boxes per §9.2.1.1.
            Display::Block => return false,
        }
    }
    has_inline
}
