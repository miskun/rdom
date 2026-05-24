//! IFC detection — is an element an inline formatting context?
//!
//! An element establishes an **IFC** when it has at least one
//! element child with `display: inline` and no element children with
//! `display: block`. Mixed (block + inline) is a cascade error —
//! we treat it as a regular block so the flex layout keeps
//! working.
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
//! **Note on `Display::InlineBlock`**: inline-block does NOT flip
//! the parent into IFC mode. An inline-block child of a flex
//! container is a regular flex item with intrinsic sizing (see
//! `flex.rs::resolve_cross_size`); it doesn't require IFC packing
//! because it has its own layout rect, and routing the parent
//! through the IFC path would zero its rect — the OOTB blocker.
//!
//! Stronger rule (M8): an inline-block child also **disqualifies**
//! the parent from IFC even when there's an inline-element sibling.
//! Previously the predicate ignored inline-block children for the
//! decision; that caused `<row><button>X</button><span>Y</span></row>`
//! (button is inline-block, span is inline) to enter IFC mode, where
//! the IFC paint path drops the button's UA `::before` / `::after`
//! pseudos because the IFC packer doesn't synthesize pseudo
//! fragments for inline-block children. Per CSS Flexbox §3 step 7
//! a flex container blockifies its children rather than establishing
//! an IFC, so falling back to flex layout for this case is closer
//! to the spec.
//!
//! Mixed inline-text + inline-block content (e.g.
//! `<p>text <button>...</button> text</p>`) is still unsupported —
//! the parent escapes IFC (no IFC text packing) but the regular
//! flex layout doesn't surface text nodes as flow content either.
//! Demos work around with the `<span>` wrapping idiom for text or
//! stack inline-block items vertically. Proper anonymous-inline-box
//! generation is deferred until a milestone after 0.1.0.

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
            Display::Inline => has_inline = true,
            // InlineBlock is inline-LEVEL per CSS but rdom routes it
            // through flex layout (with intrinsic cross-axis sizing)
            // instead of IFC packing. It also disqualifies the
            // parent from IFC even with inline-element siblings —
            // see the module-doc note for the spec rationale.
            Display::InlineBlock => return false,
            // Display::None children are invisible and don't
            // participate in layout — they don't count as inline
            // but also don't disqualify an IFC (treat like a
            // whitespace/comment child).
            Display::None => continue,
            Display::Block => return false,
        }
    }
    has_inline
}
