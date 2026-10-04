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
//! - Interaction changes (hover / active / focus) → the elements whose
//!   match flipped. `:hover`, `:active` and `:focus-within` match every
//!   ancestor of the element holding the state (Selectors 4 §9.2 /
//!   §9.4 / §13.3), so that is **the old and new ancestor chains minus
//!   their common part**, marked at each side's topmost element
//!   (`:focus-visible` flips on the focused element alone)
//! - Sibling combinators: `a:hover + b`, `[x] ~ p`, `.e:empty + p` let
//!   an element's match read a *previous sibling's* state. When a
//!   compound left of a `+` / `~` in the sheets can read the kind of
//!   change made — a pseudo-class for hover / focus / text / `:empty`,
//!   the attribute's own name (`class`, `id` included) for an attribute
//!   change (`style::sibling_triggers`, `P7G-SIBLING-MARK-NARROW-1`;
//!   everything until the App tells the tracker its sheets) — an
//!   element whose own state changed also dirties its parent's element
//!   children, once per parent per drain
//! - The parent of a tree mutation is dirtied too when its own match
//!   can change: `:empty` (its first element / text child arrived or
//!   its last one left) and, when text nodes come or go,
//!   `:placeholder-shown` / `::placeholder`, which read the text
//!   content (`Dom::is_placeholder_shown`)
//! - Character-data changes dirty only the elements whose selector
//!   state reads text, when the text went from empty to non-empty or
//!   back: the parent (its `:empty`) and an ancestor with a
//!   `placeholder` attribute
//!   (`P7G-PAINT-ONLY-FRAME-1` — the runtime no longer re-cascades the
//!   whole tree on a text edit, so the tracker must name them). Every
//!   text change also changes layout and painted output, so the tracker
//!   keeps a separate `paint_dirty` flag (consumed via
//!   `take_paint_dirty()`) for the runtime to lay out and repaint even
//!   though no cascade work is queued.
//! - Selection changes (`SelectionChanged`) dirty no element — paint
//!   reads `Dom::selection` — but set a `selection_dirty` flag
//!   (`take_selection_dirty()`) the runtime turns into a repaint.
//!
//! The App's frame prelude consumes both flags on every frame, not only
//! after an input event (`P7G-OFF-EVENT-PAINT-1`).
//!
//! ## Dedupe policy
//!
//! When marking `X` dirty, we check whether any ancestor of `X` is
//! already in the roots list. If yes, the ancestor will
//! re-cascade the whole subtree including `X`, so we skip pushing.
//! This keeps the roots list small even under bursts of mutations.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, InteractionKind, Mutation, MutationObserver, NodeId, ObserverId};

use crate::ext::TuiExt;
use crate::style::sibling_triggers::{Cause, SiblingTriggers};

/// Shared handle to the dirty-roots list. Created by
/// `DirtyTracker::install`; the tracker uses it internally, and
/// callers retrieve accumulated roots via `take_roots()`.
#[derive(Debug, Clone, Default)]
pub struct DirtyTracker {
    inner: Rc<RefCell<DirtyState>>,
    observer_id: Option<ObserverId>,
}

#[derive(Debug)]
struct DirtyState {
    roots: Vec<NodeId>,
    /// Mirror of `roots` for O(1) membership (the Vec keeps insertion
    /// order for deterministic cascade).
    roots_set: std::collections::HashSet<NodeId>,
    /// Parents whose element children were all marked dirty for
    /// sibling-dependent selectors since the last drain. A second
    /// `ChildListChanged` on the same parent before the cascade runs
    /// finds them dirty already, so the O(children) loop is skipped —
    /// appending n rows one by one is O(n) marks, not O(n²).
    sibling_marked: std::collections::HashSet<NodeId>,
    /// Text-only mutations don't affect the cascade (selectors don't
    /// match against text content) but they DO change painted output.
    /// Set by `CharacterDataChanged`; consumed by the runtime's redraw
    /// decision via `take_paint_dirty()`. Without this flag, a
    /// `set_node_value` call from inside an event handler is invisible
    /// until something else dirties the cascade.
    paint_dirty: bool,
    /// The selection or caret moved (`SelectionChanged`): paint draws
    /// the `::selection` overlay and the caret from `Dom::selection`,
    /// so only a repaint is due. Consumed via `take_selection_dirty()`.
    selection_dirty: bool,
    /// Records observed since install — evidence that code changed the
    /// tree (`records_seen`, `P7G-TICK-TOUCHED-1`).
    records: u64,
    /// Which changes can reach a sibling's match through a `+` / `~`
    /// combinator (`style::sibling_triggers`,
    /// `P7G-SIBLING-MARK-NARROW-1`). Every change until the App says
    /// otherwise.
    siblings: SiblingTriggers,
}

impl Default for DirtyState {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            roots_set: std::collections::HashSet::new(),
            sibling_marked: std::collections::HashSet::new(),
            paint_dirty: false,
            selection_dirty: false,
            records: 0,
            siblings: SiblingTriggers::all(),
        }
    }
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
        let mut state = self.inner.borrow_mut();
        state.roots_set.clear();
        state.sibling_marked.clear();
        std::mem::take(&mut state.roots)
    }

    /// Return the accumulated dirty roots, clearing the internal list.
    /// Call this right before `cascade_subtrees`.
    pub fn take_roots(&self) -> Vec<NodeId> {
        let mut state = self.inner.borrow_mut();
        state.roots_set.clear();
        state.sibling_marked.clear();
        std::mem::take(&mut state.roots)
    }

    /// Peek at the current dirty roots without clearing. Useful in
    /// tests.
    pub fn roots_snapshot(&self) -> Vec<NodeId> {
        self.inner.borrow().roots.clone()
    }

    /// Consume and return the paint-dirty flag: a mutation that queues
    /// no (or not only) cascade work but changes layout and painted
    /// output — text edits like `set_node_value`, text nodes coming or
    /// going. The App's frame prelude reads it on every frame, whoever
    /// made the change (a listener, a timer, an injected closure,
    /// `dom_mut()`), and lays out and repaints
    /// (`P7G-OFF-EVENT-PAINT-1`).
    pub fn take_paint_dirty(&self) -> bool {
        std::mem::take(&mut self.inner.borrow_mut().paint_dirty)
    }

    /// Peek at the paint-dirty flag without clearing. Useful in tests.
    pub fn paint_dirty_snapshot(&self) -> bool {
        self.inner.borrow().paint_dirty
    }

    /// Consume and return the selection-dirty flag: the selection or
    /// caret moved since the last call. Paint reads the selection
    /// directly, so the App's frame prelude repaints without laying out
    /// (`P7G-OFF-EVENT-PAINT-1`).
    pub fn take_selection_dirty(&self) -> bool {
        std::mem::take(&mut self.inner.borrow_mut().selection_dirty)
    }

    /// Whether the next frame has anything the tracker recorded to
    /// draw: dirty roots, the paint-dirty flag or the selection-dirty
    /// flag. Does not clear anything.
    #[cfg(test)]
    pub(crate) fn has_pending(&self) -> bool {
        let state = self.inner.borrow();
        !state.roots.is_empty() || state.paint_dirty || state.selection_dirty
    }

    /// How many mutation records the tracker has observed since it was
    /// installed. The App compares it around a callback to tell whether
    /// the callback changed the tree (`P7G-TICK-TOUCHED-1`).
    pub(crate) fn records_seen(&self) -> u64 {
        self.inner.borrow().records
    }

    /// Registered observer's handle. `None` after `uninstall`.
    pub fn observer_id(&self) -> Option<ObserverId> {
        self.observer_id
    }

    /// Say whether the stylesheets cascaded against this tree use a
    /// sibling combinator (`+` / `~`) anywhere
    /// ([`uses_sibling_combinators`]). While they may (the default), an
    /// element whose own state changes also dirties its siblings, whose
    /// match can read that state (`a:hover + b`); with `false` it does
    /// not, which keeps a hover move in a long list from re-cascading
    /// the list. The App sets it whenever its sheets change.
    pub fn set_sibling_combinators(&self, used: bool) {
        self.inner.borrow_mut().siblings = if used {
            SiblingTriggers::all()
        } else {
            SiblingTriggers::none()
        };
    }

    /// Say which changes can reach a sibling's match under the sheets
    /// now cascaded — the precise form of
    /// [`set_sibling_combinators`](Self::set_sibling_combinators) the
    /// App uses (`P7G-SIBLING-MARK-NARROW-1`): `h1 + p` reads nothing a
    /// change can flip, `a:hover + b` reads state, `[x] + b` reads `x`.
    pub(crate) fn set_sibling_triggers(&self, triggers: SiblingTriggers) {
        self.inner.borrow_mut().siblings = triggers;
    }

    /// Manually mark a subtree dirty. Escape hatch for cases the
    /// `MutationObserver` doesn't cover — a direct write to a `TuiExt`
    /// field the cascade reads, such as `TuiExt::set_inline_style`
    /// (which mutates the ext data, not DOM state, so no `Mutation`
    /// fires). The `TuiNodeMutExt` setters on a `NodeMut` reflect into
    /// the `style` attribute and need no manual mark.
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
        state.records = state.records.wrapping_add(1);
        match record {
            Mutation::AttributeChanged { id, name, .. } => {
                mark_state_dirty(dom, &mut state, *id, Cause::Attribute(name));
            }
            Mutation::ClassChanged { id, .. } => {
                mark_state_dirty(dom, &mut state, *id, Cause::Attribute("class"));
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
                // of the parent so :first-child / + / ~ re-evaluate. Once
                // per parent per drain — they stay dirty until the
                // cascade runs, and later-added children are marked
                // directly above.
                if state.sibling_marked.insert(*parent) {
                    let sibling_ids: Vec<NodeId> =
                        dom.node(*parent).children().map(|n| n.id()).collect();
                    for sib in sibling_ids {
                        mark_style_dirty(dom, &mut state, sib);
                    }
                }
                // Text nodes coming or going (a `<div></div>` getting a
                // text node appended, or the inverse) change layout and
                // paint: flag paint_dirty, which the runtime turns into
                // a layout + paint frame (`Redraw::Layout`) that picks
                // up the new text in intrinsic-size / flex-distribution
                // / IFC packing. Their cascade effect is the parent's
                // own match, handled below.
                let any_text_added = added
                    .iter()
                    .any(|&n| dom.node(n).node_type() == rdom_core::NodeType::Text);
                let any_text_removed = removed
                    .iter()
                    .any(|&n| dom.node(n).node_type() == rdom_core::NodeType::Text);
                let text_changed = any_text_added || any_text_removed;
                if text_changed {
                    state.paint_dirty = true;
                }
                // The parent's own match: `:empty` flips when it had no
                // element / text children before (all of them are the
                // added ones) or has none now — both mean at most
                // `added.len()` such children remain — and text coming
                // or going can flip `:placeholder-shown` on it and on
                // any ancestor carrying a placeholder.
                // Children that count for `:empty` (Selectors 4 §14.2):
                // elements and non-empty text.
                let emptiness_may_flip = dom
                    .node(*parent)
                    .child_nodes()
                    .filter(|c| match c.node_type() {
                        rdom_core::NodeType::Element => true,
                        rdom_core::NodeType::Text => c.node_value().is_some_and(|t| !t.is_empty()),
                        _ => false,
                    })
                    .nth(added.len())
                    .is_none();
                if emptiness_may_flip || text_changed {
                    mark_state_dirty(dom, &mut state, *parent, Cause::State);
                }
                if text_changed {
                    mark_placeholder_hosts(dom, &mut state, *parent);
                }
            }
            Mutation::CharacterDataChanged { id, old, new } => {
                // Selectors do not match text, but `:placeholder-shown`
                // (and the `::placeholder` it gates) reads whether the
                // text content is empty: dirty the placeholder hosts
                // above when that flipped. Every text change also lays
                // out and paints anew: flag paint-dirty so the runtime
                // does, even though no cascade roots are queued.
                // A text node going empty ↔ non-empty can also flip its
                // parent's `:empty` (a zero-length text node does not
                // count, Selectors 4 §14.2).
                if old.is_empty() != new.is_empty()
                    && let Some(parent) = dom.node(*id).parent_node().map(|p| p.id())
                {
                    mark_state_dirty(dom, &mut state, parent, Cause::State);
                    mark_placeholder_hosts(dom, &mut state, parent);
                }
                state.paint_dirty = true;
            }
            Mutation::InteractionChanged { prev, next, kind } => {
                crate::rdom_trace!(
                    "DirtyTracker::observe InteractionChanged kind={kind:?} prev={prev:?} next={next:?}; \
                     marking style_dirty + pushing roots"
                );
                match kind {
                    // `:hover`, `:active` and `:focus-within` match the
                    // element holding the state and every ancestor of it
                    // (Selectors 4 §9.2 / §9.4 / §13.3); `:focus` rides
                    // along with `:focus-within`.
                    InteractionKind::Hover | InteractionKind::Focus | InteractionKind::Active => {
                        mark_chain_change(dom, &mut state, *prev, *next);
                    }
                    // `:focus-visible` flips on the focused element alone
                    // (`prev == next`).
                    _ => {
                        for id in [prev, next].into_iter().flatten() {
                            mark_state_dirty(dom, &mut state, *id, Cause::State);
                        }
                    }
                }
                crate::rdom_trace!(
                    "DirtyTracker::observe InteractionChanged: roots now = {:?}",
                    state.roots
                );
            }
            Mutation::SelectionChanged { .. } => {
                // Selection changes don't affect cascade — the
                // `::selection` pseudo-element overlay and the caret
                // are applied by paint directly from `dom.selection()`,
                // not via the style cascade. They do change painted
                // output, whoever moved the selection (a script's
                // `select()`, a timer): flag a repaint.
                state.selection_dirty = true;
            }
            Mutation::PreDetach { .. } => {
                // Cascade-relevant state changes (focused / hovered
                // clearing to None) fire their own
                // `InteractionChanged` record from the purge step;
                // PreDetach itself is a pure event-pipeline hook
                // and doesn't carry any cascade implication.
            }
            // `Mutation` is `#[non_exhaustive]`. A record kind added
            // upstream must get an arm here; until it does, repaint and
            // trip the assert in the workspace's tests.
            other => {
                debug_assert!(false, "DirtyTracker: unhandled mutation {other:?}");
                state.paint_dirty = true;
            }
        }
    }
}

/// Mark `from` and each of its ancestors that carries a non-empty
/// `placeholder` attribute — the elements whose `:placeholder-shown`
/// reads the text below them. O(depth), no allocation.
fn mark_placeholder_hosts(dom: &mut Dom<TuiExt>, state: &mut DirtyState, from: NodeId) {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let node = dom.node(id);
        cur = node.parent_node().map(|p| p.id());
        if node
            .get_attribute("placeholder")
            .is_some_and(|v| !v.is_empty())
        {
            mark_state_dirty(dom, state, id, Cause::State);
        }
    }
}

/// An ancestor-matched state (`:hover`, `:active`, `:focus-within`)
/// moved from `prev` to `next`: the elements whose match flipped are
/// the two inclusive ancestor chains minus their common part. Each
/// side's topmost flipped element is marked — its subtree cascade
/// covers the rest of that side's chain, and `mark_state_dirty` reaches
/// its siblings for `+` / `~` — so moving between two children of a
/// hovered `<li>` restyles the two children, not the `<li>`.
/// O(depth), two small allocations.
fn mark_chain_change(
    dom: &mut Dom<TuiExt>,
    state: &mut DirtyState,
    prev: Option<NodeId>,
    next: Option<NodeId>,
) {
    let chain = |dom: &Dom<TuiExt>, from: Option<NodeId>| {
        let mut chain: Vec<NodeId> = Vec::new();
        let mut cur = from;
        while let Some(id) = cur {
            let node = dom.node(id);
            if node.ext().is_some() {
                chain.push(id);
            }
            cur = node.parent_node().map(|p| p.id());
        }
        chain
    };
    let mut old = chain(dom, prev);
    let mut new = chain(dom, next);
    // Both chains end at the same root when both nodes are connected:
    // drop the shared tail, whose match does not change.
    while !old.is_empty() && old.last() == new.last() {
        old.pop();
        new.pop();
    }
    for top in [old.last(), new.last()].into_iter().flatten() {
        mark_state_dirty(dom, state, *top, Cause::State);
    }
}

/// `id`'s own selector state changed (`cause`): mark its subtree, and —
/// when some `+` / `~` in the sheets can read that kind of change
/// (`SiblingTriggers::fires`) — its parent's element children, whose
/// match can read `id`'s state through the sibling combinator. The
/// children are marked once per parent per drain (`sibling_marked`).
fn mark_state_dirty(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId, cause: Cause<'_>) {
    mark_style_dirty(dom, state, id);
    if !state.siblings.fires(cause) {
        return;
    }
    let Some(parent) = dom.node(id).parent_node().map(|p| p.id()) else {
        return;
    };
    if !state.sibling_marked.insert(parent) {
        return;
    }
    let mut sib = dom.node(parent).first_element_child().map(|c| c.id());
    while let Some(s) = sib {
        mark_style_dirty(dom, state, s);
        sib = dom.node(s).next_element_sibling().map(|c| c.id());
    }
}

/// Whether any selector `sheet` matches with has a sibling combinator
/// (`+` / `~`) anywhere — in a rule's selector or an `@scope`'s start /
/// end, inside `:not()` / `:is()` / `:where()` too
/// (`style::selector_walk`) — the input of
/// [`DirtyTracker::set_sibling_combinators`].
pub fn uses_sibling_combinators(sheet: &crate::style::Stylesheet) -> bool {
    use crate::style::selector_walk::{any_complex, sheet_selectors};
    use rdom_core::selectors::Combinator;
    sheet_selectors(sheet).any(|c| {
        any_complex(c, &mut |c| {
            c.ancestors.iter().any(|(comb, _)| {
                matches!(
                    comb,
                    Combinator::AdjacentSibling | Combinator::GeneralSibling
                )
            })
        })
    })
}

/// Mark `id`'s subtree as dirty. Sets `style_dirty=true` on the node
/// itself and pushes it to the roots worklist — unless an ancestor is
/// already a queued root (the ancestor's cascade will re-cascade us).
fn mark_style_dirty(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId) {
    // Non-element nodes (text/comment/fragment root) don't have a TuiExt
    // and don't participate in the cascade directly. But their parent
    // might — we just skip them here.
    if dom.node(id).ext().is_none() {
        return;
    }

    // Walk ancestors. If any ancestor is a queued root, its cascade
    // covers us — don't push to roots, but still flip our flag for
    // completeness. The test is roots-set membership, not the
    // ancestor's `style_dirty` flag: a flag set without a queued root
    // (a direct `TuiExt` write, a stale flag) would otherwise swallow
    // every root below it (`P7G-SETTER-MUTATION-1`).
    let mut ancestor_dirty = false;
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(a) = cur {
        if state.roots_set.contains(&a) {
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
        if state.roots_set.insert(id) {
            state.roots.push(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, TuiDom, TuiNodeExt, TuiNodeMutExt, TuiStyle};

    /// `P7G-PAINT-ONLY-FRAME-1`: a text edit dirties the placeholder host
    /// above it and its parent only when the text went from empty to
    /// non-empty (or back) — the selector states text feeds
    /// (`:placeholder-shown`, and `:empty`, `P7G-CORE-SMALL-1`) — and
    /// always flags paint-dirty.
    #[test]
    fn text_emptiness_flips_dirty_the_placeholder_host() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let ta = dom.create_element("textarea");
        dom.set_attribute(ta, "placeholder", "p").unwrap();
        let t = dom.create_text_node("");
        dom.append_child(ta, t).unwrap();
        dom.append_child(root, ta).unwrap();
        let plain = dom.create_element("p");
        let pt = dom.create_text_node("");
        dom.append_child(plain, pt).unwrap();
        dom.append_child(root, plain).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        // No `+` / `~` in play: only the host itself restyles.
        tracker.set_sibling_combinators(false);

        dom.node_mut(t).set_node_value("a").unwrap();
        assert_eq!(tracker.take_roots(), vec![ta], "empty → text");
        dom.node_mut(ta).ext_mut().unwrap().style_dirty = false;
        assert!(tracker.take_paint_dirty());

        dom.node_mut(t).set_node_value("ab").unwrap();
        assert!(tracker.take_roots().is_empty(), "text → text: no restyle");
        assert!(tracker.take_paint_dirty(), "but a relayout");

        dom.node_mut(t).set_node_value("").unwrap();
        assert_eq!(tracker.take_roots(), vec![ta], "text → empty");

        // No placeholder above, but the parent's `:empty` flips: a
        // zero-length text node does not count (Selectors 4 §14.2).
        dom.node_mut(pt).set_node_value("x").unwrap();
        assert_eq!(tracker.take_roots(), vec![plain], "the parent's :empty");
        dom.node_mut(plain).ext_mut().unwrap().style_dirty = false;

        dom.node_mut(pt).set_node_value("xy").unwrap();
        assert!(tracker.take_roots().is_empty(), "text → text: no restyle");
    }

    /// `P7G-ROUTE-REDRAW-1`: `a[x] + b` reads `a`'s attribute, so while
    /// the sheets may use a sibling combinator an attribute change on
    /// `a` dirties `b` too; told they do not, only `a`'s subtree.
    #[test]
    fn a_state_change_dirties_siblings_only_while_sibling_combinators_may_apply() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let a = dom.create_element("div");
        let b = dom.create_element("p");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        let tracker = DirtyTracker::install(&mut dom);

        dom.set_attribute(a, "x", "1").unwrap();
        let roots = tracker.take_roots();
        assert!(roots.contains(&a) && roots.contains(&b), "{roots:?}");
        for id in [a, b] {
            dom.node_mut(id).ext_mut().unwrap().style_dirty = false;
        }

        tracker.set_sibling_combinators(false);
        dom.set_attribute(a, "x", "2").unwrap();
        assert_eq!(tracker.take_roots(), vec![a]);
    }

    #[test]
    fn sibling_combinators_are_found_anywhere_in_a_selector() {
        use crate::style::Stylesheet;
        let uses = |sel: &str| {
            uses_sibling_combinators(&Stylesheet::bare().rule_unchecked(sel, TuiStyle::new()))
        };
        assert!(uses("a + b"));
        assert!(uses("a ~ b c"));
        assert!(uses("p:not(a + b)"));
        assert!(uses(":where(a ~ b) > c"));
        assert!(!uses("a > b c"));
        assert!(!uses("p:not(.x)"));
        assert!(
            !uses_sibling_combinators(&Stylesheet::new()),
            "the UA sheet has none"
        );
    }

    /// `C1G-INVALIDATION`: an `@scope`'s `<scope-start>` / `<scope-end>`
    /// (CSS Cascade 6 §2.5) and an `:is()` argument (the nesting `&`,
    /// CSS Nesting 1 §2) are matched too.
    #[test]
    fn sibling_combinators_are_found_in_scope_preludes_and_is() {
        use rdom_style::{RuleContext, StyleSelector, Stylesheet};
        let uses = |css: &str| uses_sibling_combinators(&rdom_css::parse(css).stylesheet);
        assert!(uses("@scope (.a + .b) { p { color: red } }"));
        assert!(uses("@scope (main) to (.a ~ .b) { p { color: red } }"));
        assert!(!uses("@scope (main) to (.b) { p { color: red } }"));
        let parent = StyleSelector::parse(".a + .b, .q + .b").unwrap();
        let mut sheet = Stylesheet::bare();
        sheet.add_style_rule(
            &StyleSelector::parse_nested("p", &parent).unwrap(),
            TuiStyle::new(),
            RuleContext::default(),
        );
        assert!(uses_sibling_combinators(&sheet), "inside `:is()`");
    }

    /// Appending children one at a time marks each parent's siblings
    /// once per drain (they stay dirty until the cascade), and a drain
    /// re-arms the sibling marking so `:first-child` / `+` / `~`
    /// re-evaluate after the next structural change.
    #[test]
    fn sibling_marking_is_once_per_parent_per_drain_and_rearms_after_drain() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let list = dom.create_element("ul");
        dom.append_child(root, list).unwrap();
        // A non-empty list: appending to an empty one would also dirty
        // the list itself (its `:empty` flips), covering every child.
        let first = dom.create_element("li");
        dom.append_child(list, first).unwrap();
        let tracker = DirtyTracker::install(&mut dom);

        let mut items = Vec::new();
        for _ in 0..2000 {
            let li = dom.create_element("li");
            dom.append_child(list, li).unwrap();
            items.push(li);
        }
        for &li in &items {
            assert!(dom.node(li).ext().unwrap().style_dirty);
        }
        let roots = tracker.take_roots();
        assert_eq!(
            roots.len(),
            2001,
            "each appended child is its own root, plus the sibling marked once"
        );
        assert_eq!(
            roots.iter().collect::<std::collections::HashSet<_>>().len(),
            2001,
            "no duplicate roots"
        );

        // Simulate the cascade clearing the flags.
        for &li in &items {
            dom.node_mut(li).ext_mut().unwrap().style_dirty = false;
        }
        // A structural change after the drain re-marks the siblings.
        let extra = dom.create_element("li");
        dom.append_child(list, extra).unwrap();
        assert!(
            dom.node(items[0]).ext().unwrap().style_dirty,
            "siblings re-marked after drain"
        );
        assert!(dom.node(extra).ext().unwrap().style_dirty);
    }

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

    /// Clear every `style_dirty` flag, as a cascade pass would, so the
    /// next record's marks can be read on their own.
    fn settle(dom: &mut TuiDom, tracker: &DirtyTracker, nodes: [NodeId; 4]) {
        tracker.take_roots();
        for id in std::iter::once(dom.root()).chain(nodes) {
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.style_dirty = false;
            }
        }
    }

    /// `root > ul > li > (s1, s2)`, a tracker with no sibling
    /// combinators in play.
    fn hover_chain() -> (TuiDom, DirtyTracker, [NodeId; 4]) {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let ul = dom.create_element("ul");
        let li = dom.create_element("li");
        let s1 = dom.create_element("span");
        let s2 = dom.create_element("span");
        dom.append_child(root, ul).unwrap();
        dom.append_child(ul, li).unwrap();
        dom.append_child(li, s1).unwrap();
        dom.append_child(li, s2).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        tracker.set_sibling_combinators(false);
        (dom, tracker, [ul, li, s1, s2])
    }

    fn dirty(dom: &TuiDom, id: NodeId) -> bool {
        dom.node(id).ext().is_some_and(|e| e.style_dirty)
    }

    /// Whether a subtree cascade of `roots` restyles `id`.
    fn covered(dom: &TuiDom, roots: &[NodeId], id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(n) = cur {
            if roots.contains(&n) {
                return true;
            }
            cur = dom.node(n).parent_node().map(|p| p.id());
        }
        false
    }

    /// `P7G-HOVER-ANCESTORS-1`: `:hover` matches the hovered element's
    /// ancestors (Selectors 4 §9.2), so entering a child restyles the
    /// whole chain it joins.
    #[test]
    fn hovering_a_child_restyles_its_ancestor_chain() {
        let (mut dom, tracker, [ul, li, s1, s2]) = hover_chain();
        dom.set_hovered(Some(s1));
        let roots = tracker.take_roots();
        assert_eq!(roots, vec![ul], "the topmost element that became hovered");
        assert!([ul, li, s1].iter().all(|&id| covered(&dom, &roots, id)));
        assert!(!dirty(&dom, s2), "a sibling is not marked on its own");
    }

    /// Moving between two children of the hovered `<li>` leaves the
    /// shared part of the chain (`li` and up) hovered: only the two
    /// children restyle.
    #[test]
    fn moving_between_siblings_restyles_only_the_unshared_chain() {
        let (mut dom, tracker, nodes @ [ul, li, s1, s2]) = hover_chain();
        dom.set_hovered(Some(s1));
        settle(&mut dom, &tracker, nodes);
        dom.set_hovered(Some(s2));
        assert_eq!(tracker.take_roots(), vec![s1, s2]);
        assert!(
            !dirty(&dom, li) && !dirty(&dom, ul),
            "the shared chain keeps :hover"
        );
    }

    /// Moving from a child to its parent: the parent stays hovered, only
    /// the child leaves the chain.
    #[test]
    fn moving_from_a_child_to_its_parent_restyles_only_the_child() {
        let (mut dom, tracker, nodes @ [_, li, s1, _]) = hover_chain();
        dom.set_hovered(Some(s1));
        settle(&mut dom, &tracker, nodes);
        dom.set_hovered(Some(li));
        assert_eq!(tracker.take_roots(), vec![s1]);
        assert!(!dirty(&dom, li));
    }

    /// Leaving the document restyles the whole old chain.
    #[test]
    fn leaving_restyles_the_whole_old_chain() {
        let (mut dom, tracker, nodes @ [ul, li, s1, _]) = hover_chain();
        dom.set_hovered(Some(s1));
        settle(&mut dom, &tracker, nodes);
        dom.set_hovered(None);
        let roots = tracker.take_roots();
        assert_eq!(roots, vec![ul]);
        assert!(covered(&dom, &roots, li) && covered(&dom, &roots, s1));
    }

    /// `:active` (Selectors 4 §9.4) follows the same chain.
    #[test]
    fn active_changes_restyle_the_unshared_chain() {
        let (mut dom, tracker, nodes @ [ul, li, s1, s2]) = hover_chain();
        dom.set_active(Some(s1));
        assert_eq!(tracker.take_roots(), vec![ul]);
        settle(&mut dom, &tracker, nodes);
        dom.set_active(Some(s2));
        assert_eq!(tracker.take_roots(), vec![s1, s2]);
        assert!(!dirty(&dom, li));
        settle(&mut dom, &tracker, nodes);
        dom.set_active(None);
        assert_eq!(tracker.take_roots(), vec![ul]);
    }

    /// Removing the hovered element clears hover while its old ancestors
    /// are still reachable, so their `:hover` restyles.
    #[test]
    fn removing_the_hovered_element_restyles_its_old_ancestors() {
        let (mut dom, tracker, nodes @ [_, li, s1, _]) = hover_chain();
        dom.set_hovered(Some(s1));
        settle(&mut dom, &tracker, nodes);
        dom.remove_child(li, s1).unwrap();
        let roots = tracker.take_roots();
        assert!(
            covered(&dom, &roots, li),
            "li no longer contains the hovered element: {roots:?}"
        );
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
        // `P7G-SETTER-MUTATION-1`: `TuiNodeMutExt::set_inline_style`
        // (like every direct style setter) reflects into the `style`
        // attribute, so the tracker sees an `AttributeChanged` and
        // queues the element — no manual `mark_dirty` needed.
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(div)
            .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
        assert_eq!(tracker.take_roots(), vec![div]);
    }

    /// `P7G-SETTER-MUTATION-1`: the dedupe skips a node only when an
    /// ancestor is a queued root. An ancestor whose `style_dirty` flag is
    /// set without being queued (a stale flag, a direct `TuiExt` write)
    /// no longer swallows the roots below it.
    #[test]
    fn an_unqueued_dirty_flag_above_does_not_swallow_a_root() {
        let mut dom: TuiDom = TuiDom::new();
        let outer = dom.create_element("div");
        let inner = dom.create_element("div");
        dom.append_child(dom.root(), outer).unwrap();
        dom.append_child(outer, inner).unwrap();
        let tracker = DirtyTracker::install(&mut dom);
        dom.node_mut(outer).ext_mut().unwrap().style_dirty = true;
        dom.set_attribute(inner, "data-x", "1").unwrap();
        assert!(tracker.take_roots().contains(&inner));
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
