//! `HitTestExt` — point → element lookup, the foundation of mouse
//! routing.
//!
//! `Dom::hit_test(x, y)` returns the deepest element whose painted
//! area contains `(x, y)`. `hit_test_path(x, y)` returns the full
//! ancestor chain (outer → inner), matching the browser's
//! `composedPath()` for a synthetic `MouseEvent` at that point.
//!
//! ## Algorithm
//!
//! The walk mirrors paint in reverse (`render::stacking`, CSS 2.1
//! Appendix E). For a stacking context: try the child contexts with
//! positive `z-index` (highest first), then the `z-index: auto | 0`
//! layer in reverse tree order, then the root's in-flow content
//! (children in reverse document order, or the inline fragment under
//! the point for an inline formatting context), then the negative
//! layer, and finally the root's own box. Every layer entry carries
//! the clip that applies to it (CSS 2.1 §11.1.1), so a positioned box
//! clipped by an ancestor's overflow is not hittable outside it, and
//! an element that clips its children ends the descent when the point
//! sits on its padding or border.
//!
//! The path is the full DOM ancestor chain of the hit, root-most
//! first, whichever layer found it — the ancestors between a context
//! root and a layer entry are inserted when the entry hits.
//!
//! ## `pointer-events`
//!
//! `pointer-events: none` makes an element transparent: it is never
//! the hit target, its subtree is still searched for `auto`
//! descendants, and otherwise the point falls through to what is
//! beneath. Inherited, like the web.

//!
//! ## Module map
//!
//! - [`descend`] — the stacking-context walk behind `hit_test_path`:
//!   layers, plain boxes, in-flow content, inline-fragment owners,
//!   `pointer-events` transparency.
//! - [`nearest`] — `InlineTarget` and the choice of inline-flow target
//!   for a text position: containment (`inline_target_at`) and the
//!   empty-space nearest-by-distance fallback.
//! - [`fragment`] — resolving a cell inside a chosen target to a
//!   `Position`: fragment lookup, line clamp, grapheme cell → byte.
//!
//! This file keeps the [`HitTestExt`] trait and its impl — the thin
//! orchestration over those three.

mod descend;
mod fragment;
mod nearest;

use rdom_core::{Dom, NodeId, Position};

use crate::ext::TuiExt;
use crate::render::Rect;
use crate::runtime::selection::user_select;

use descend::hit_stacking_context;
use nearest::inline_target_at;

pub(crate) use fragment::resolve_in_target;
pub(crate) use nearest::nearest_inline_target_in_subtree;

/// Extension trait adding hit-test lookup to `Dom<TuiExt>`.
pub trait HitTestExt {
    /// The deepest element whose painted area contains `(x, y)`.
    /// Uses the last-painted-wins rule: when two siblings overlap,
    /// the later one wins. Returns `None` if no element covers the
    /// point (e.g., empty viewport).
    ///
    /// For IFC blocks the lookup descends into the inline layout so
    /// a point landing on text inside a `<code>` fragment returns
    /// the `<code>` element, not the enclosing `<p>`.
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId>;

    /// The full ancestor chain from root to the deepest hit, in
    /// document order (root-most first, deepest last). Suitable for
    /// event-dispatch targets or browser-style `composedPath()`
    /// walks. Empty when nothing hit.
    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId>;

    /// Map a screen cell `(x, y)` to a DOM text position — a
    /// `(text_node, byte_offset)` pair suitable for
    /// [`Dom::set_selection`].
    ///
    /// A point in empty space — a gap between blocks, above/below all
    /// content, or a non-IFC container whose text lives in descendants —
    /// **snaps to the nearest text position** by vertical distance
    /// (scoped to the deepest hit element's subtree), matching how
    /// browsers resolve `caretPositionFromPoint` and drag-select past
    /// content. Without this, dragging past the bottom edge collapsed
    /// the selection back to the anchor block.
    ///
    /// Returns `None` when:
    /// - `(x, y)` misses every element AND the document root subtree
    ///   has no inline flow to snap to;
    /// - the innermost hit element or one of its ancestors has
    ///   `user-select: none` (chrome, buttons, etc. — the
    ///   selection algorithm skips these subtrees); the nearest-flow
    ///   fallback likewise skips `user-select: none` candidates.
    ///
    /// The returned `offset` is a byte offset into the text
    /// node's data — matches the `Selection` / `Range` API and
    /// Rust string slicing conventions.
    fn position_at(&self, x: u16, y: u16) -> Option<Position>;

    /// The nearest **selectable** text position to `(x, y)` by vertical
    /// distance — `position_at`'s empty-space resolution, exposed for the
    /// drag-extend fallback. Unlike `position_at` it never returns `None`
    /// for a `user-select: none` hit: it skips those candidates and snaps to
    /// the closest selectable flow instead (so dragging a selection *over* a
    /// `user-select: none` bar extends to the text beyond it rather than
    /// collapsing). Scoped to the deepest hit element's subtree, escalating
    /// up the ancestor chain until a subtree has selectable text. `None` only
    /// when nothing selectable exists to snap to.
    fn nearest_selectable_position(&self, x: u16, y: u16) -> Option<Position>;
}

impl HitTestExt for Dom<TuiExt> {
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId> {
        self.hit_test_path(x, y).last().copied()
    }

    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId> {
        let mut path = Vec::new();
        // Reverse paint order through the stacking contexts (CSS 2.1
        // Appendix E; `render::stacking`). Hit-testing has no viewport
        // of its own: overflow ancestors are the only clips.
        let unclipped = Rect::new(0, 0, u16::MAX, u16::MAX);
        hit_stacking_context(self, self.root(), x, y, unclipped, unclipped, &mut path);
        path
    }

    fn position_at(&self, x: u16, y: u16) -> Option<Position> {
        // Find the inline-flow container under (x, y) — either a
        // classic IFC block (singular `inline_layout`) or one of
        // a parent's anonymous block boxes (BFC-1 phase 3.3). The
        // hit-test path is walked innermost-first; the deepest
        // matching container wins.
        // `hit_test_path` already handles overflow clipping and
        // reverse-document-order
        // stacking; we just need to find the first IFC ancestor
        // on the path.
        let path = self.hit_test_path(x, y);

        // Walk path *innermost-first* — deepest match wins. A
        // singular IFC block (its own `inline_layout`) is the
        // common case; an anonymous block box (a slot in some
        // ancestor's `anonymous_blocks` Vec, populated by the
        // block-layout pass for inline runs amongst block
        // children) is the BFC-1 phase 3 case.
        if let Some(target) = path
            .iter()
            .rev()
            .find_map(|&id| inline_target_at(self, id, y))
        {
            // user-select gate: any ancestor of the hit with
            // `user-select: none` kills the position.
            if user_select::is_unselectable(self, *path.last()?) {
                return None;
            }
            return resolve_in_target(self, target, x, y);
        }

        // Empty space inside a `user-select: none` subtree (e.g. a table row's
        // trailing space past its last cell): no caret. Do NOT snap out to the
        // nearest selectable text elsewhere — that would start a text-selection
        // drag from a non-selectable region and hijack the consumer (the grid).
        // This mirrors the contained-case gate above. The drag-extend path
        // (`nearest_selectable_position`) is unaffected — it deliberately skips
        // *over* user-select:none regions to keep extending an existing drag.
        if let Some(&deepest) = path.last()
            && user_select::is_unselectable(self, deepest)
        {
            return None;
        }

        // No inline-flow target contains `y`: the point is in empty space —
        // a gap between blocks, above/below all content, or a non-IFC
        // container with text only in descendants. Browsers snap
        // `caretPositionFromPoint` (and drag-select) to the nearest text
        // position rather than returning nothing.
        self.nearest_selectable_position(x, y)
    }

    fn nearest_selectable_position(&self, x: u16, y: u16) -> Option<Position> {
        // Resolve to the closest inline-flow target by vertical distance.
        // Start scoped to the deepest hit element's subtree (so a Page-
        // scrollport gap resolves within that page, not unrelated chrome) and
        // escalate up the ancestor chain until a subtree has text — e.g. a hit
        // landing in an empty sibling spacer, or on a `user-select: none` bar,
        // climbs to the parent that also holds the prose. The search skips
        // `user-select: none` candidates, so the snap never lands on chrome.
        let path = self.hit_test_path(x, y);
        let mut scope = path.last().copied();
        loop {
            let id = scope.unwrap_or_else(|| self.root());
            if let Some(target) = nearest_inline_target_in_subtree(self, id, y) {
                return resolve_in_target(self, target, x, y);
            }
            if id == self.root() {
                return None;
            }
            scope = self.node(id).parent_node().map(|p| p.id());
        }
    }
}

#[cfg(test)]
mod tests;
