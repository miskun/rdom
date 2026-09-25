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
//! 2. Inherit the inherited properties (`rdom_style::property_dispatch::inherits`) from the parent
//!    (`inherit`).
//! 3. Collect matching rules via `rdom_core::Dom::matches_list`.
//! 4. Sort candidates by (specificity, source_idx). Ascending =
//!    late-wins.
//! 5. Apply declarations in origin + importance order (`apply`):
//!    1. UA normal, Author normal, Inline normal,
//!    2. Inline important, Author important, UA important.
//!
//!    Within each ladder step, sort by (specificity, source_idx).
//! 6. Resolve `Value::Inherit` / `Value::Initial` per-property.
//! 7. Resolve `content` (`content`) — pseudo-element body.
//! 8. Finalize `border_fg` (fall back to final `fg`).
//! 9. Write to `TuiExt.computed` and flip `style_dirty=false`; if
//!    any layout-affecting property's new value differs, set
//!    `layout_dirty=true` (`inherit::layout_differs`).
//!
//! Pseudo-elements use the same algorithm but start from the host's
//! computed style (not the parent's). They contribute a concrete
//! `content: Option<String>` resolved from `TuiStyle.content` plus any
//! fallback `before_content` / `after_content` set directly on
//! `TuiExt`.
//!
//! ## Module layout
//!
//! - `walk` — `cascade_subtree`, `compute_element_style`,
//!   `compute_pseudo_style`. The tree recursion lives here.
//! - `apply` — cascade ladder + per-property applicators.
//! - `inherit` — `inherit_inheritable_from`, `layout_differs`.
//! - `content` — pseudo-element `content` resolution.
//!
//! ## Inheritance model
//!
//! Which properties inherit is a fact about the property, declared once
//! in `rdom_style::property_dispatch::inherits` (it also decides what
//! `unset` means). `inherit::inherit_inheritable_from` copies exactly
//! that set from parent to child, and a cascade test probes every
//! property against the table so the two cannot drift.

mod apply;
mod content;
mod counters;
mod inherit;
mod walk;

#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Content, Stylesheet};

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
    fn cascade_all(&mut self, stylesheets: &[&Stylesheet]);

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
    fn cascade_subtrees_all(&mut self, stylesheets: &[&Stylesheet], roots: &[NodeId]);
}

impl CascadeExt for Dom<TuiExt> {
    fn cascade(&mut self, stylesheet: &Stylesheet) {
        self.cascade_all(&[stylesheet]);
    }

    fn cascade_all(&mut self, stylesheets: &[&Stylesheet]) {
        let merged_vars = walk::merge_root_vars(stylesheets);
        let root = self.root();
        // The root's parent carries the sheet-level (`define_var` /
        // `:root`) variables; every element then inherits its parent's
        // map and layers its own declarations on top.
        let mut parent = ComputedStyle::initial();
        parent.vars = merged_vars.clone();
        // Full-tree cascade: `tree_has_positioned_pseudo` flags get
        // written authoritatively, top-to-bottom. No bubble-up needed
        // because the walk visits every ancestor.
        let mut counters = walk::CounterState::default();
        let _ = walk::cascade_subtree(self, stylesheets, root, &parent, &mut counters);
    }

    fn cascade_subtrees(&mut self, stylesheet: &Stylesheet, roots: &[NodeId]) {
        self.cascade_subtrees_all(&[stylesheet], roots);
    }

    fn cascade_subtrees_all(&mut self, stylesheets: &[&Stylesheet], roots: &[NodeId]) {
        let merged_vars = walk::merge_root_vars(stylesheets);
        let uses_counters = stylesheets.iter().any(|s| {
            s.rules().iter().any(|r| {
                r.style.counter_reset.is_some()
                    || r.style.counter_increment.is_some()
                    || r.style
                        .content
                        .as_ref()
                        .and_then(|c| c.as_specified())
                        .is_some_and(Content::uses_counters)
            })
        });
        // A queued root can have been FREED between when it was marked
        // dirty and now: dropping one child fires `ChildListChanged`, whose
        // dirty-tracker handler marks every remaining sibling dirty (sibling
        // selectors), and one of those siblings may itself be dropped later
        // in the same teardown. A freed node has no subtree to cascade —
        // skip it rather than dereferencing a reclaimed arena slot.
        let mut live: Vec<NodeId> = roots
            .iter()
            .copied()
            .filter(|r| self.contains(*r))
            .collect();
        if live.is_empty() {
            return;
        }
        if uses_counters {
            // Counters make every root depend on everything before it in
            // tree order. One pre-order walk carries the state, replays the
            // stored ops of untouched elements and cascades each root when
            // the walk reaches it, so an earlier root is recomputed before a
            // later root's counters are read: O(N) for any number of roots,
            // and correct after insertions (a fresh node has no stored ops
            // to replay — its own cascade supplies them).
            // Only roots inside the document take part: a detached
            // subtree that was marked dirty (the previous demo of a
            // swap, a removed row) renders nothing, and the walk from the
            // document root would never reach it — it must not sit at the
            // head of the queue and starve every root behind it.
            let root = self.root();
            live.retain(|r| {
                *r == root
                    || self
                        .compare_document_position(root, *r)
                        .contains(rdom_core::DocumentPosition::CONTAINED_BY)
            });
            live.sort_by(|a, b| tree_order(self, *a, *b));
            live.dedup();
            let mut next = 0usize;
            let mut counters = walk::CounterState::default();
            walk::cascade_roots_in_order(
                self,
                stylesheets,
                &merged_vars,
                &live,
                &mut next,
                root,
                &mut counters,
            );
            return;
        }
        for root in live {
            let parent_computed = walk::parent_computed_for(self, root, &merged_vars);
            let mut counters = walk::CounterState::default();
            let flags =
                walk::cascade_subtree(self, stylesheets, root, &parent_computed, &mut counters);
            walk::bubble_subtree_flags(self, root, flags);
        }
    }
}

/// Tree order (DOM §4.2.1) for two live nodes; equal only for the same node.
fn tree_order(dom: &Dom<TuiExt>, a: NodeId, b: NodeId) -> std::cmp::Ordering {
    use rdom_core::DocumentPosition;
    use std::cmp::Ordering;
    if a == b {
        return Ordering::Equal;
    }
    let pos = dom.compare_document_position(a, b);
    if pos.contains(DocumentPosition::FOLLOWING) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

// ─── Small helper re-exported for test support ──────────────────────

/// Quick probe: the computed style of `id`, or `initial()` if none
/// (pre-cascade, or non-element). Useful for tests.
pub fn computed_of(dom: &Dom<TuiExt>, id: NodeId) -> ComputedStyle {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_deref().cloned())
        .unwrap_or_else(ComputedStyle::initial)
}
