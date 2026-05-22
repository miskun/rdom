//! The cascade engine.
//!
//! `Dom::cascade(&stylesheet)` walks the tree top-down, computes a
//! `ComputedStyle` for every element (and any matching `::before` /
//! `::after` pseudo-elements), and writes the result back to each
//! `TuiExt`. Dirty flags get cleared; `layout_dirty` gets set whenever
//! a layout-affecting property value changes.
//!
//! ## Algorithm (per element)
//!
//! 1. Start from `ComputedStyle::initial()`.
//! 2. Inherit the subset of properties in `INHERITS_MASK` from parent
//!    ([`inherit`]).
//! 3. Collect matching rules via `rdom_core::Dom::matches_list`.
//! 4. Sort candidates by (specificity, source_idx). Ascending =
//!    late-wins.
//! 5. Apply declarations in origin + importance order ([`apply`]):
//!    1. UA normal, Author normal, Inline normal,
//!    2. Inline important, Author important, UA important.
//!
//!    Within each ladder step, sort by (specificity, source_idx).
//! 6. Resolve `Value::Inherit` / `Value::Initial` per-property.
//! 7. Resolve `content` ([`content`]) — pseudo-element body.
//! 8. Finalize `border_fg` (fall back to final `fg`).
//! 9. Write to `TuiExt.computed` and flip `style_dirty=false`; if
//!    any layout-affecting property's new value differs, set
//!    `layout_dirty=true` ([`inherit::layout_differs`]).
//!
//! Pseudo-elements use the same algorithm but start from the host's
//! computed style (not the parent's). They contribute a concrete
//! `content: Option<String>` resolved from `TuiStyle.content` plus any
//! fallback `before_content` / `after_content` set directly on
//! `TuiExt`.
//!
//! ## Module layout
//!
//! - [`masks`] — `PropMask` bitfield + `INHERITS_MASK` + `LAYOUT_MASK`.
//! - [`walk`] — `cascade_subtree`, `compute_element_style`,
//!   `compute_pseudo_style`. The tree recursion lives here.
//! - [`apply`] — cascade ladder + per-property applicators.
//! - [`inherit`] — `inherit_inheritable_from`, `layout_differs`.
//! - [`content`] — pseudo-element `content` resolution.
//!
//! ## Inheritance model
//!
//! Which properties inherit is declared once in [`INHERITS_MASK`] as a
//! `PropMask` bitfield. Adding a new inheritable property is a
//! single-line change in [`masks`], not a code path rewrite.

mod apply;
mod content;
mod inherit;
mod masks;
mod walk;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Stylesheet};

pub use masks::{INHERITS_MASK, LAYOUT_MASK, PropMask};

// ─── Public entry point ─────────────────────────────────────────────

/// Extension trait adding the cascade methods to `Dom<TuiExt>`.
/// Lives in rdom-tui so `Dom` in rdom-core stays style-agnostic. Users
/// pull it in with `use rdom_tui::CascadeExt;` (or via
/// `use rdom_tui::*;`).
///
/// Each method comes in two forms: the single-`Stylesheet` form for
/// ergonomic use in tests and the rare app with one sheet, and the
/// `&[Stylesheet]` form that the runtime uses when an `App` has
/// multiple sheets registered (`push_stylesheet` / `set_stylesheet` /
/// construction). Within the slice, later sheets win same-specificity
/// contests — push order is the tiebreaker, matching `Document.styleSheets`
/// ordering on the web. The single-sheet form is a thin wrapper around
/// the slice form with a one-element slice.
pub trait CascadeExt {
    /// Cascade the whole document against `stylesheet`. Writes
    /// `ComputedStyle` entries to every element's `TuiExt`, clears
    /// `style_dirty`, sets `layout_dirty` on elements whose
    /// layout-affecting property values changed. Use for initial
    /// paint or after a stylesheet swap.
    fn cascade(&mut self, stylesheet: &Stylesheet);

    /// Multi-sheet variant of [`Self::cascade`]. Rules are merged
    /// across all sheets; later sheets win same-specificity contests.
    /// Custom-property (`var()`) definitions are merged with
    /// later-wins semantics per var name.
    fn cascade_all(&mut self, stylesheets: &[Stylesheet]);

    /// Cascade only the subtrees rooted at `roots`. Each root's
    /// parent is consulted for inheritance (so a root's computed fg
    /// still inherits correctly from its ancestor chain). Empty list
    /// = no-op.
    ///
    /// Use after incremental mutations: pair with `DirtyTracker` to
    /// get the list of roots that actually need re-cascade. The
    /// resulting performance scales with the size of changed
    /// subtrees, not the whole tree.
    fn cascade_subtrees(&mut self, stylesheet: &Stylesheet, roots: &[NodeId]);

    /// Multi-sheet variant of [`Self::cascade_subtrees`].
    fn cascade_subtrees_all(&mut self, stylesheets: &[Stylesheet], roots: &[NodeId]);
}

impl CascadeExt for Dom<TuiExt> {
    fn cascade(&mut self, stylesheet: &Stylesheet) {
        self.cascade_all(std::slice::from_ref(stylesheet));
    }

    fn cascade_all(&mut self, stylesheets: &[Stylesheet]) {
        let root = self.root();
        let parent = ComputedStyle::initial();
        // Full-tree cascade: `tree_has_positioned_pseudo` flags get
        // written authoritatively, top-to-bottom. No bubble-up needed
        // because the walk visits every ancestor.
        let _ = walk::cascade_subtree(self, stylesheets, root, &parent);
    }

    fn cascade_subtrees(&mut self, stylesheet: &Stylesheet, roots: &[NodeId]) {
        self.cascade_subtrees_all(std::slice::from_ref(stylesheet), roots);
    }

    fn cascade_subtrees_all(&mut self, stylesheets: &[Stylesheet], roots: &[NodeId]) {
        for &root in roots {
            // Look up parent's computed style for inheritance. Root
            // has no parent, or the parent is a Fragment/root — use
            // initial in either case.
            let parent_computed = self
                .node(root)
                .parent_node()
                .and_then(|p| p.ext().and_then(|e| e.computed.clone()))
                .unwrap_or_else(ComputedStyle::initial);
            let flags = walk::cascade_subtree(self, stylesheets, root, &parent_computed);
            // If the partial cascade introduced a positioned pseudo
            // or a `border-collapse: collapse` element anywhere in
            // the subtree, bubble `true` up through ancestors so the
            // document-level early-exit check doesn't stale-`false`
            // and miss it. We never bubble `false` — that would
            // require seeing all ancestor siblings to know whether
            // any other subtree still has the flag set.
            if flags.has_positioned_pseudo || flags.has_collapse {
                let mut cur = self.node(root).parent_node().map(|p| p.id());
                while let Some(p) = cur {
                    if let Some(ext) = self.node_mut(p).ext_mut() {
                        if flags.has_positioned_pseudo {
                            ext.tree_has_positioned_pseudo = true;
                        }
                        if flags.has_collapse {
                            ext.tree_has_collapse = true;
                        }
                    }
                    cur = self.node(p).parent_node().map(|n| n.id());
                }
            }
        }
    }
}

// ─── Small helper re-exported for test support ──────────────────────

/// Quick probe: the computed style of `id`, or `initial()` if none
/// (pre-cascade, or non-element). Useful for tests.
pub fn computed_of(dom: &Dom<TuiExt>, id: NodeId) -> ComputedStyle {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.clone())
        .unwrap_or_else(ComputedStyle::initial)
}
