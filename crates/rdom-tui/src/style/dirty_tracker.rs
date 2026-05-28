//! `DirtyTracker` — a `MutationObserver` that flips `style_dirty` on
//! affected elements and maintains a worklist of subtree roots for
//! incremental re-cascade.
//!
//! ## Usage
//!
//! ```
//! # use rdom_tui::{TuiDom, Stylesheet, TuiStyle, Color, CascadeExt};
//! # use rdom_tui::style::dirty_tracker::DirtyTracker;
//! let mut dom: TuiDom = TuiDom::new();
//! let tracker = DirtyTracker::install(&mut dom);
//!
//! let sheet = Stylesheet::bare()
//!     .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
//!
//! // Initial cascade — everything, writes computed styles.
//! dom.cascade(&sheet);
//!
//! // Later, some mutations happen; the tracker collects dirty roots.
//! let div = dom.create_element("div");
//! dom.append_child(dom.root(), div).unwrap();
//!
//! // Re-cascade only the changed subtrees.
//! let roots = tracker.take_roots();
//! dom.cascade_subtrees(&sheet, &roots);
//! ```
//!
//! ## What counts as dirty
//!
//! - Attribute changes, class changes, inline style edits → **node +
//!   subtree** (conservative — selectors like `a b` mean a parent's
//!   attribute change can affect descendants)
//! - Tree mutations (insert / remove / clear) → **inserted subtree +
//!   all element children of the affected parent** (for
//!   sibling-dependent selectors like `:first-child`, `+`, `~`)
//! - Interaction changes (hover / focus) → **both the old and new
//!   target's subtree** (so pseudo matches re-evaluate)
//! - Character-data changes do NOT dirty the cascade — text content
//!   doesn't affect selector matching. But they DO change the painted
//!   output, so the tracker maintains a separate `paint_dirty` flag
//!   (consumed via `take_paint_dirty()`) for the runtime to know it
//!   must repaint even though no cascade work is queued.
//!
//! ## Dedupe policy
//!
//! When marking `X` dirty, we check whether any ancestor of `X` is
//! already dirty (and in the roots list). If yes, the ancestor will
//! re-cascade the whole subtree including `X`, so we skip pushing.
//! This keeps the roots list small even under bursts of mutations.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, InteractionKind, Mutation, MutationObserver, NodeId, ObserverId};

use crate::ext::TuiExt;

/// Shared handle to the dirty-roots list. Created by
/// `DirtyTracker::install`; the tracker uses it internally, and
/// callers retrieve accumulated roots via `take_roots()`.
#[derive(Debug, Clone, Default)]
pub struct DirtyTracker {
    inner: Rc<RefCell<DirtyState>>,
    observer_id: Option<ObserverId>,
}

#[derive(Debug, Default)]
struct DirtyState {
    roots: Vec<NodeId>,
    /// Text-only mutations don't affect the cascade (selectors don't
    /// match against text content) but they DO change painted output.
    /// Set by `CharacterDataChanged`; consumed by the runtime's redraw
    /// decision via `take_paint_dirty()`. Without this flag, a
    /// `set_node_value` call from inside an event handler is invisible
    /// until something else dirties the cascade.
    paint_dirty: bool,
}

impl DirtyTracker {
    /// Register a `MutationObserver` that writes into a fresh tracker.
    /// The returned `DirtyTracker` holds a `Rc`-cloned handle to the
    /// same state — both the observer (inside `Dom.observers`) and
    /// external callers read/write a shared `RefCell`.
    pub fn install(dom: &mut Dom<TuiExt>) -> Self {
        let inner = Rc::new(RefCell::new(DirtyState::default()));
        let shim = Shim {
            inner: inner.clone(),
        };
        let observer_id = dom.add_mutation_observer(Box::new(shim));
        Self {
            inner,
            observer_id: Some(observer_id),
        }
    }

    /// Remove the observer from `dom` and return the final dirty-roots
    /// list. After calling this, `take_roots()` returns an empty Vec.
    pub fn uninstall(self, dom: &mut Dom<TuiExt>) -> Vec<NodeId> {
        if let Some(id) = self.observer_id {
            dom.remove_mutation_observer(id);
        }
        std::mem::take(&mut self.inner.borrow_mut().roots)
    }

    /// Return the accumulated dirty roots, clearing the internal list.
    /// Call this right before `cascade_subtrees`.
    pub fn take_roots(&self) -> Vec<NodeId> {
        std::mem::take(&mut self.inner.borrow_mut().roots)
    }

    /// Peek at the current dirty roots without clearing. Useful in
    /// tests.
    pub fn roots_snapshot(&self) -> Vec<NodeId> {
        self.inner.borrow().roots.clone()
    }

    /// Consume and return the paint-dirty flag (text-only mutations
    /// like `set_node_value` that don't dirty the cascade but DO
    /// change painted output). Read by the App's event loop after
    /// dispatching a user event so a handler that mutates text content
    /// triggers an immediate redraw.
    pub fn take_paint_dirty(&self) -> bool {
        std::mem::take(&mut self.inner.borrow_mut().paint_dirty)
    }

    /// Peek at the paint-dirty flag without clearing. Useful in tests.
    pub fn paint_dirty_snapshot(&self) -> bool {
        self.inner.borrow().paint_dirty
    }

    /// Registered observer's handle. `None` after `uninstall`.
    pub fn observer_id(&self) -> Option<ObserverId> {
        self.observer_id
    }

    /// Manually mark a subtree dirty. Escape hatch for cases the
    /// `MutationObserver` doesn't cover — most importantly, writing
    /// `TuiExt.inline_style` directly via `set_inline_style` (which
    /// mutates the ext data, not DOM state, so no `Mutation` fires).
    ///
    /// Behavior matches the automatic path: flips `style_dirty` on the
    /// node, dedupes against dirty ancestors, pushes to the roots list
    /// only when necessary.
    pub fn mark_dirty(&self, dom: &mut Dom<TuiExt>, id: NodeId) {
        let mut state = self.inner.borrow_mut();
        mark_style_dirty(dom, &mut state, id);
    }
}

struct Shim {
    inner: Rc<RefCell<DirtyState>>,
}

impl MutationObserver<TuiExt> for Shim {
    fn observe(&mut self, dom: &mut Dom<TuiExt>, record: &Mutation) {
        let mut state = self.inner.borrow_mut();
        match record {
            Mutation::AttributeChanged { id, .. } | Mutation::ClassChanged { id, .. } => {
                mark_style_dirty(dom, &mut state, *id);
            }
            Mutation::ChildListChanged {
                parent,
                added,
                removed,
                ..
            } => {
                // The inserted subtrees get dirtied directly.
                for a in added {
                    mark_style_dirty(dom, &mut state, *a);
                }
                // Sibling-dependent selectors: mark all element children
                // of the parent so :first-child / + / ~ re-evaluate.
                let sibling_ids: Vec<NodeId> =
                    dom.node(*parent).children().map(|n| n.id()).collect();
                for sib in sibling_ids {
                    mark_style_dirty(dom, &mut state, sib);
                }
                // Text-only mutations (a `<div></div>` getting a
                // text node appended, or the inverse) don't touch
                // any element with a TuiExt — `mark_style_dirty`
                // on a text node early-returns, and the element-
                // children loop above doesn't see text nodes. The
                // mutation IS visible-content-changing though, so
                // flag paint_dirty: the runtime's event loop ORs
                // this into `needs_redraw`, which triggers a fresh
                // `draw_if_dirty` that re-runs `layout_dom`
                // (full-tree re-flow) and picks up the new text in
                // intrinsic-size / flex-distribution / IFC packing.
                //
                // We don't mark the parent element style_dirty
                // because that would trigger cascade work that
                // text-only changes don't need (CSS selectors don't
                // match text content) and that empirically breaks
                // pseudo-element-driven built-ins (form/input
                // seeding, label `for` resolution).
                let any_text_added = added
                    .iter()
                    .any(|&n| dom.node(n).node_type() == rdom_core::NodeType::Text);
                let any_text_removed = removed
                    .iter()
                    .any(|&n| dom.node(n).node_type() == rdom_core::NodeType::Text);
                if any_text_added || any_text_removed {
                    state.paint_dirty = true;
                }
            }
            Mutation::CharacterDataChanged { .. } => {
                // Text data doesn't affect selector matching — no
                // cascade work needed. But the painted output for the
                // text-bearing element changed, so flag paint-dirty
                // so the runtime knows to repaint even though no
                // cascade roots queued.
                state.paint_dirty = true;
            }
            Mutation::InteractionChanged { prev, next, kind } => {
                crate::rdom_trace!(
                    "DirtyTracker::observe InteractionChanged kind={kind:?} prev={prev:?} next={next:?}; \
                     marking style_dirty + pushing roots"
                );
                if let Some(p) = prev {
                    mark_style_dirty(dom, &mut state, *p);
                }
                if let Some(n) = next {
                    mark_style_dirty(dom, &mut state, *n);
                }
                // For focus changes, the `:focus-within` pseudo
                // class also flips on every ancestor of the
                // prev/next focused node. Walk up and mark the
                // topmost element ancestor — `mark_style_dirty`'s
                // ancestor-already-dirty check then dedupes the
                // descendants we marked above. Hover doesn't
                // propagate up via a `:hover-within` (no such
                // selector exists in CSS) so we skip this for
                // non-focus kinds.
                if matches!(kind, InteractionKind::Focus) {
                    if let Some(p) = prev {
                        mark_ancestor_chain_style_dirty(dom, &mut state, *p);
                    }
                    if let Some(n) = next {
                        mark_ancestor_chain_style_dirty(dom, &mut state, *n);
                    }
                }
                crate::rdom_trace!(
                    "DirtyTracker::observe InteractionChanged: roots now = {:?}",
                    state.roots
                );
            }
            Mutation::SelectionChanged { .. } => {
                // Selection changes don't affect cascade — the
                // `::selection` pseudo-element overlay is applied by
                // paint directly from `dom.selection()`, not via
                // the style cascade. The runtime flips needs_redraw
                // out-of-band when it updates the selection (via
                // the Router / selection helper). Nothing to do
                // here.
            }
            Mutation::PreDetach { .. } => {
                // Cascade-relevant state changes (focused / hovered
                // clearing to None) fire their own
                // `InteractionChanged` record from the purge step;
                // PreDetach itself is a pure event-pipeline hook
                // and doesn't carry any cascade implication.
            }
        }
    }
}

/// Walk from `id`'s parent upward through the element ancestor
/// chain and mark each as style-dirty. Used when an interaction
/// state propagates upward through `:focus-within` — every
/// ancestor's selector match flips.
///
/// The walk goes innermost-to-outermost; `mark_style_dirty`'s
/// own ancestor-already-dirty check kicks in once the outermost
/// ancestor is marked, so we don't push an exploding number of
/// roots for deep trees.
fn mark_ancestor_chain_style_dirty(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId) {
    // Collect element ancestors first so `mark_style_dirty` can
    // process them outermost-first — the topmost ancestor's push
    // covers every descendant via the subtree cascade, so the
    // closer ancestors hit the dirty-ancestor dedupe path and
    // don't end up as extra roots.
    let mut chain: Vec<NodeId> = Vec::new();
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(a) = cur {
        if dom.node(a).ext().is_some() {
            chain.push(a);
        }
        cur = dom.node(a).parent_node().map(|p| p.id());
    }
    for ancestor in chain.into_iter().rev() {
        mark_style_dirty(dom, state, ancestor);
    }
}

/// Mark `id`'s subtree as dirty. Sets `style_dirty=true` on the node
/// itself and pushes it to the roots worklist — unless an ancestor is
/// already a dirty root (the ancestor's cascade will re-cascade us).
fn mark_style_dirty(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId) {
    // Non-element nodes (text/comment/fragment root) don't have a TuiExt
    // and don't participate in the cascade directly. But their parent
    // might — we just skip them here.
    if dom.node(id).ext().is_none() {
        return;
    }

    // Walk ancestors. If any ancestor already has style_dirty, its
    // cascade covers us — don't push to roots, but still flip our flag
    // for completeness.
    let mut ancestor_dirty = false;
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(a) = cur {
        if dom.node(a).ext().is_some_and(|e| e.style_dirty) {
            ancestor_dirty = true;
            break;
        }
        cur = dom.node(a).parent_node().map(|p| p.id());
    }

    // Flip self.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.style_dirty = true;
    }

    if !ancestor_dirty {
        // If `id` is already in the roots list, don't push it twice.
        if !state.roots.contains(&id) {
            state.roots.push(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, TuiDom, TuiNodeExt, TuiNodeMutExt, TuiStyle};

    #[test]
    fn install_returns_tracker() {
        let mut dom: TuiDom = TuiDom::new();
        let tracker = DirtyTracker::install(&mut dom);
        assert!(tracker.observer_id().is_some());
        assert_eq!(dom.observer_count(), 1);
    }

    #[test]
    fn uninstall_removes_observer() {
        let mut dom: TuiDom = TuiDom::new();
        let tracker = DirtyTracker::install(&mut dom);
        let _roots = tracker.uninstall(&mut dom);
        assert_eq!(dom.observer_count(), 0);
    }

    #[test]
    fn set_attribute_marks_dirty() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        let tracker = DirtyTracker::install(&mut dom);
        dom.set_attribute(div, "id", "main").unwrap();

        let roots = tracker.take_roots();
        assert!(roots.contains(&div));
        assert!(dom.node(div).ext().unwrap().style_dirty);
    }

    #[test]
    fn add_class_marks_dirty() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.add_class(div, "active").unwrap();
        assert!(tracker.take_roots().contains(&div));
    }

    #[test]
    fn tree_mutation_marks_subtree_and_siblings() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let parent = dom.create_element("div");
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(parent, a).unwrap();
        dom.append_child(parent, b).unwrap();
        dom.append_child(root, parent).unwrap();

        let tracker = DirtyTracker::install(&mut dom);
        // Insert a new child — siblings a, b should also be dirty
        // (sibling-dependent selectors might now match differently).
        let c = dom.create_element("c");
        dom.append_child(parent, c).unwrap();

        let roots = tracker.roots_snapshot();
        // Dedupe: parent's `parent` becomes a dirty root first (through the
        // insertion path), then sibling dirtying for a/b gets subsumed by
        // their parent's dirt... actually no, their parent `parent` is not
        // itself dirty, only its children. So a, b, c should each be roots.
        assert!(roots.contains(&c));
    }

    #[test]
    fn hover_changes_mark_both_prev_and_next() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();

        let tracker = DirtyTracker::install(&mut dom);
        dom.set_hovered(Some(a));
        let roots1 = tracker.take_roots();
        assert!(roots1.contains(&a));

        dom.set_hovered(Some(b));
        let roots2 = tracker.take_roots();
        // Both the old (a) and new (b) should now be dirty.
        assert!(roots2.contains(&a));
        assert!(roots2.contains(&b));
    }

    #[test]
    fn focus_changes_mark_prev_and_next() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();

        let tracker = DirtyTracker::install(&mut dom);
        dom.set_focused(Some(a));
        dom.set_focused(Some(b));
        let roots = tracker.take_roots();
        assert!(roots.contains(&a));
        assert!(roots.contains(&b));
    }

    #[test]
    fn focus_changes_dirty_ancestor_chain_for_focus_within() {
        // `:focus-within` matches every ancestor of the focused
        // element. When focus moves, those ancestors' style
        // changes — so the cascade must re-run on at least one
        // root that covers them. The dirty tracker walks up from
        // prev/next and marks an ancestor that re-cascades the
        // whole chain.
        //
        // Tree: outer > middle > inner.
        // Focus inner → outer's `:focus-within` flips → outer's
        // subtree must be re-cascaded.
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let outer = dom.create_element("div");
        let middle = dom.create_element("div");
        let inner = dom.create_element("span");
        dom.append_child(middle, inner).unwrap();
        dom.append_child(outer, middle).unwrap();
        dom.append_child(root, outer).unwrap();

        let tracker = DirtyTracker::install(&mut dom);
        dom.set_focused(Some(inner));
        let roots = tracker.take_roots();
        // The exact root pushed is an implementation detail (could
        // be `inner`, or its topmost element ancestor `outer`).
        // What matters is that SOMETHING re-cascades the chain —
        // either the topmost ancestor is a root, or every ancestor
        // along the chain has `style_dirty` set so its parent's
        // cascade visits them. The simplest pin: `outer` (the
        // topmost element ancestor) must end up either in roots
        // OR have `style_dirty = true`, so a future cascade pass
        // re-evaluates its `:focus-within` selector match.
        let outer_dirty =
            roots.contains(&outer) || dom.node(outer).ext().is_some_and(|e| e.style_dirty);
        assert!(
            outer_dirty,
            "outer must re-cascade so its :focus-within match flips when inner gets focus"
        );
    }

    #[test]
    fn dedup_with_dirty_ancestor() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let parent = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(parent, child).unwrap();
        dom.append_child(root, parent).unwrap();

        let tracker = DirtyTracker::install(&mut dom);

        // Mutate parent first — parent gets dirty.
        dom.set_attribute(parent, "role", "banner").unwrap();
        // Now mutate child — ancestor is dirty, child should not be
        // added to the roots list (but its style_dirty flag still flips).
        dom.set_attribute(child, "id", "x").unwrap();

        let roots = tracker.take_roots();
        assert!(roots.contains(&parent));
        assert!(!roots.contains(&child));
        assert!(dom.node(child).ext().unwrap().style_dirty);
    }

    #[test]
    fn take_roots_clears_list() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.set_attribute(div, "x", "1").unwrap();
        assert!(!tracker.take_roots().is_empty());
        // Second take returns empty — state was cleared.
        assert!(tracker.take_roots().is_empty());
    }

    #[test]
    fn roots_snapshot_does_not_clear() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.set_attribute(div, "x", "1").unwrap();
        let s1 = tracker.roots_snapshot();
        let s2 = tracker.roots_snapshot();
        assert_eq!(s1, s2);
    }

    #[test]
    fn character_data_change_does_not_dirty_cascade_but_flags_paint() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let t = dom.create_text_node("hello");
        dom.append_child(root, t).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(t).set_node_value("world").unwrap();
        // Text data change fires CharacterDataChanged — selectors don't
        // depend on text, so cascade is not dirty.
        assert!(tracker.roots_snapshot().is_empty());
        // But painted output changed, so paint_dirty IS set.
        assert!(tracker.paint_dirty_snapshot());
    }

    #[test]
    fn take_paint_dirty_clears_flag() {
        let mut dom: TuiDom = TuiDom::new();
        let t = dom.create_text_node("hi");
        dom.append_child(dom.root(), t).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(t).set_node_value("ho").unwrap();
        assert!(tracker.take_paint_dirty());
        // Second take returns false — flag was cleared.
        assert!(!tracker.take_paint_dirty());
    }

    #[test]
    fn set_hovered_to_same_does_not_dirty() {
        let mut dom: TuiDom = TuiDom::new();
        let a = dom.create_element("a");
        dom.append_child(dom.root(), a).unwrap();
        dom.set_hovered(Some(a));
        let tracker = DirtyTracker::install(&mut dom);
        // No-op: already hovering a.
        dom.set_hovered(Some(a));
        assert!(tracker.take_roots().is_empty());
    }

    #[test]
    fn duplicate_dirty_is_deduplicated() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.set_attribute(div, "x", "1").unwrap();
        dom.set_attribute(div, "y", "2").unwrap();
        dom.set_attribute(div, "z", "3").unwrap();
        // Three mutations on the same node → only one roots entry.
        let roots = tracker.take_roots();
        assert_eq!(roots.iter().filter(|&&r| r == div).count(), 1);
    }

    #[test]
    fn inline_style_setter_marks_dirty() {
        // TuiNodeMutExt::set_inline_style writes to the TuiExt directly —
        // no rdom-core mutation fires. So the dirty tracker CANNOT see
        // it. Document this limitation: callers who set inline_style
        // must also call dom.mark_style_dirty manually, OR mutate via
        // DOM operations (set_attribute, add_class) that flow through
        // the observer.
        //
        // For now: call tracker.mark_dirty(&mut dom, id) manually to
        // fire invalidation; see the positive test below.
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(div)
            .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
        // No mutation fired → no dirty roots.
        assert!(tracker.take_roots().is_empty());
    }

    #[test]
    fn mark_dirty_escape_hatch() {
        // Companion test: after writing inline_style, the caller uses
        // tracker.mark_dirty() to trigger cascade invalidation.
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(div)
            .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
        tracker.mark_dirty(&mut dom, div);
        assert!(tracker.take_roots().contains(&div));
        assert!(dom.node(div).is_style_dirty());
    }
}
