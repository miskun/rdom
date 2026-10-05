//! IFC detection — is an element an inline formatting context?
//!
//! An element establishes an **IFC** when at least one of its
//! element children is an `inline flow` box (Display::Inline with
//! the `flow` inner type) — atomic inlines (`inline-block`,
//! `inline-flex`) neither establish nor prevent one — AND no children are
//! block-level. Mixed block + inline is a cascade error here — the
//! block-layout pass handles it via anonymous block boxes (CSS
//! 2.1 §9.2.1.1, see `render/layout_pass/block.rs`).
//!
//! **Pure-text blocks (`<note>only text</note>`) are deliberately
//! NOT IFC.** They're routed through the non-IFC paint path
//! (`paint_inline_content`), which also owns chrome substitution
//! (gauges, closed dropdowns, password masks) and the no-text
//! single-row fallback. Both kinds still pack their text — and their
//! static `::before` / `::after` — through `compute_inline_layout`, so
//! wrap, caret, hit-test and paint share one geometry.
//!
//! **Display::InlineBlock in IFC** (BFC-1 phase 3.5b): an
//! inline-block child participates in IFC as an atomic inline-
//! level box (CSS 2.1 §10.8) — the IFC packer emits one fragment
//! per inline-block carrying the box's intrinsic width and rows (its
//! line box grows to hold it, CSS 2.1 §10.8), and paint paints it as a
//! box at that rect, at its turn in the line (C5G-ATOM-BOX). UA pseudo
//! content (`<button>`'s `[ ]` brackets) shows through.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Flow};

/// True iff `id`'s children establish an inline formatting context.
/// Used by both layout (to skip flex distribution + populate
/// `inline_layout`) and paint (to switch to fragment-driven paint).
///
/// **Flex containers are NEVER IFC**, per CSS Flexbox §3: inline-level
/// children of a flex container are *blockified* — their display
/// computes to a block-level equivalent and they participate in the
/// flex layout as ordinary flex items. Without this carve-out, a
/// `<div style="display: flex"><span>A</span><span>B</span></div>`
/// would silently route through the IFC path, each `<span>` would
/// get a zero-sized layout rect, and the flex layout the author asked
/// for would never run. Bug surfaced by the showcase status bar's
/// two-slot pattern (hints left + mouse position right).
pub(crate) fn is_ifc_block(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    // Flex or grid container → never an IFC. Its inline children
    // blockify.
    let parent_flow = dom
        .node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.flow)
        .unwrap_or(Flow::Block);
    if parent_flow.is_flex_or_grid() {
        return false;
    }

    let mut has_inline = false;
    for child in dom.node(id).child_nodes() {
        if child.node_type() != NodeType::Element {
            continue;
        }
        let computed = child.ext().and_then(|e| e.computed.as_ref());
        // A float is out of flow (CSS 2.1 §9.5): neither inline content
        // nor a block-level box of this flow — the packer places it.
        if computed.is_some_and(|c| c.float != crate::layout::Float::None)
            && super::float::float_side(dom, child.id()).is_some()
        {
            continue;
        }
        // An atomic inline (`inline-block`, `inline-flex`, CSS Display 3
        // §2.4) is one opaque box in the line, never inline text.
        if computed.is_some_and(|c| crate::render::box_tree::is_atomic_inline(c)) {
            continue;
        }
        let display = computed.map(|c| c.display).unwrap_or(Display::Block);
        match display {
            // `Inline` (an `inline flow` box) triggers IFC: its text
            // packs into the parent's inline flow.
            Display::Inline => has_inline = true,
            // An inline block is an atomic inline, skipped above: it
            // neither triggers nor disqualifies an IFC. Beside an
            // `Inline` sibling the packer makes it an atom of this
            // context's lines; alone or only with text the block pass
            // packs it in an anonymous block box's line.
            Display::InlineBlock => continue,
            // Display::None children are invisible and don't
            // participate in layout — they don't count as inline
            // but also don't disqualify an IFC (treat like a
            // whitespace/comment child).
            Display::None => continue,
            // A box-less child (CSS Display 3 §2.5) is its content: a
            // block box in it makes this a block container with
            // anonymous boxes; otherwise it is inline content.
            Display::Contents if crate::render::box_tree::holds_block_box(dom, child.id()) => {
                return false;
            }
            Display::Contents => has_inline = true,
            // Block-level child → not IFC. The block-layout pass
            // will partition into anonymous boxes per §9.2.1.1.
            Display::Block => return false,
        }
    }
    has_inline
}
