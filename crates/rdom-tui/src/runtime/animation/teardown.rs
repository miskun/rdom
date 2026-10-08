//! Cancelling what runs on a box that went away (CSS Transitions 1 §3,
//! CSS Animations 1 §4.1, Web Animations 1 §5.6): an element taken out of
//! the document has its transitions and CSS animations cancelled —
//! `transitioncancel` / `animationcancel` — and, being no longer
//! rendered, no before-change style, so one inserted again starts afresh
//! (C12G-DETACHED).
//!
//! The removals reach here from the mutation path (the dirty tracker's
//! `take_detached`), so a frame pays for what was removed, not for the
//! tree; the per-frame check over the registry's own entries
//! ([`AnimationRegistry::cancel_disconnected`]) catches a node that left
//! by a path no record names.

use std::collections::HashSet;
use std::time::Instant;

use rdom_core::{Dom, NodeId, NodeType};

use super::AnimationRegistry;
use crate::ext::TuiExt;

impl AnimationRegistry {
    /// The frame's removals: cancel every transition and animation in
    /// each removed subtree still in the arena, and forget its
    /// before-change styles and composited values. Run before the
    /// frame's cascade, so a subtree inserted again in the same task is
    /// diffed as newly rendered.
    pub(crate) fn detach(&mut self, dom: &mut Dom<TuiExt>, roots: &[NodeId], now: Instant) {
        let removed: HashSet<NodeId> = roots.iter().copied().filter(|&r| dom.contains(r)).collect();
        if removed.is_empty() {
            return;
        }
        // One climb per running element against every removed root at
        // once: O(entries × depth), whatever the number of roots (a sort
        // moves every row).
        for node in self.nodes() {
            if !dom.contains(node) {
                continue;
            }
            let mut cur = Some(subject(dom, node));
            while let Some(n) = cur {
                #[cfg(test)]
                TEARDOWN_STEPS.with(|c| c.set(c.get() + 1));
                if removed.contains(&n) {
                    self.cancel_for_node(node, now);
                    break;
                }
                cur = dom.node(n).parent_node().map(|p| p.id());
            }
        }
        let mut forgotten = HashSet::with_capacity(removed.len());
        for &root in roots {
            if removed.contains(&root) && forgotten.insert(root) {
                let connected = dom.node(root).is_connected();
                forget_subtree(dom, root, connected);
            }
        }
    }

    /// Cancel the transitions and animations of every element that is no
    /// longer in the document (detached by a path no removal record
    /// reaches), and drop, silently, those of elements no longer in the
    /// arena. O(entries × depth): the registry's own entries, no tree
    /// walk.
    pub(super) fn cancel_disconnected(&mut self, dom: &Dom<TuiExt>, now: Instant) {
        for node in self.nodes() {
            if dom.contains(node) && !dom.node(subject(dom, node)).is_connected() {
                self.cancel_for_node(node, now);
            }
        }
        self.active.retain(|a| dom.contains(a.node));
        self.css.retain(|a| dom.contains(a.node));
    }

    /// The distinct elements something runs on.
    fn nodes(&self) -> Vec<NodeId> {
        let mut out: Vec<NodeId> = self
            .active
            .iter()
            .map(|a| a.node)
            .chain(self.css.iter().map(|a| a.node))
            .chain(self.custom_nodes())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

#[cfg(test)]
thread_local! {
    /// The steps `detach` took to find what runs under the removed roots
    /// (cost tests).
    pub(crate) static TEARDOWN_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The node whose place in the document `node`'s is: a
/// `::details-content` box, which is outside the tree, stands where its
/// `<details>` does.
fn subject(dom: &Dom<TuiExt>, node: NodeId) -> NodeId {
    crate::render::box_tree::slot::host_of(dom, node).unwrap_or(node)
}

/// Forget the rendering state of the elements of the box tree under
/// `root` (`TuiExt::forget_rendering`).
fn forget_subtree(dom: &mut Dom<TuiExt>, root: NodeId, connected: bool) {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if dom.node(id).node_type() != NodeType::Element {
            continue;
        }
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.forget_rendering(connected);
        }
        stack.extend(crate::render::box_tree::children(dom, id));
    }
}
