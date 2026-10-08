//! The tracker's `MutationObserver`: each mutation record, the elements
//! it dirties.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, InteractionKind, Mutation, MutationObserver, NodeId};

use super::DirtyState;
use super::marks::{
    mark_auto_direction_host, mark_chain_change, mark_column_change, mark_has_anchors,
    mark_placeholder_hosts, mark_state_dirty, mark_style_dirty,
};
use crate::ext::TuiExt;
use crate::style::sibling_triggers::Cause;

pub(super) struct Shim {
    pub(super) inner: Rc<RefCell<DirtyState>>,
}

impl MutationObserver<TuiExt> for Shim {
    fn observe(&mut self, dom: &mut Dom<TuiExt>, record: &Mutation) {
        let mut state = self.inner.borrow_mut();
        state.records = state.records.wrapping_add(1);
        match record {
            Mutation::AttributeChanged { id, name, .. } => {
                mark_state_dirty(dom, &mut state, *id, Cause::Attribute(name));
                if state.columns && matches!(name.as_str(), "span" | "colspan" | "rowspan") {
                    mark_column_change(dom, &mut state, *id);
                }
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
                // A removed element's transitions and animations are
                // cancelled at the next frame (C12G-DETACHED).
                state.detached.extend(
                    removed
                        .iter()
                        .copied()
                        .filter(|&n| dom.node(n).node_type() == rdom_core::NodeType::Element),
                );
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
                // New or departed content can hold a `dir=auto` host's
                // first strong character.
                mark_auto_direction_host(dom, &mut state, *parent);
                // A `:has()` anchor above (or, for `+` / `~`, before)
                // the change can gain or lose its match (C11-HAS). The
                // parent's children — the earlier siblings of what came
                // or went — are marked above already.
                if state.has.any() {
                    mark_has_anchors(dom, &mut state, *parent, true);
                }
                // Rows, cells and columns coming or going move cells
                // between an HTML table's columns.
                if state.columns {
                    mark_column_change(dom, &mut state, *parent);
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
                // Any edit can change a `dir=auto` host's first strong
                // character (HTML §3.2.6.4).
                if let Some(parent) = dom.node(*id).parent_node().map(|p| p.id()) {
                    mark_auto_direction_host(dom, &mut state, parent);
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
            Mutation::HighlightsChanged => {
                // The registered highlights (CSS Custom Highlight API 1)
                // paint as overlays read from `dom.highlights()`, as the
                // selection does: a repaint, no cascade.
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
