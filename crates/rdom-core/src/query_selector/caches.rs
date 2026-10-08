//! Caches one selector-matching pass shares across the elements it
//! matches ([`SelectorCaches`]).
//!
//! `:nth-child()` and its family read an element's index among its
//! siblings. Counting siblings per match costs O(siblings), so matching
//! every child of a long list is quadratic; like Blink's and Servo's
//! nth-index caches, the first match under a parent indexes all of the
//! parent's children for that kind of count, and every later match reads
//! the index — O(siblings) per parent and kind for the whole pass.
//!
//! `:has()` (Selectors 4 §4.5) searches an anchor's subtree or later
//! siblings. Its answers are kept per (relative selector, scoping root,
//! anchor); for a downward argument (descendant and child combinators
//! only) the search records, per element and compound, whether a match
//! of the rest lies below it, so the anchors nested in one another reuse
//! it — a deep chain of anchors costs one walk per compound, not one per
//! anchor.
//!
//! Entries are keyed by node, by the scoping root where a selector can
//! read `:scope`, and — for `of S` — by the address of the
//! `S` selector list, so the caches are valid only while the tree and
//! the selectors they were built for are unchanged. The tree half is
//! enforced: every mutation record moves the `Dom`'s mutation epoch, and
//! caches built under another epoch are dropped before use. The
//! selectors half is the caller's: one `SelectorCaches` per pass over
//! one set of parsed selectors.

use std::collections::{HashMap, HashSet};

use crate::Directionality;
use crate::node_id::NodeId;

/// The caches of one selector-matching pass. Create one per pass (a
/// cascade, a query) and hand it to every
/// [`Dom::matches_list_with`](crate::Dom::matches_list_with) call of the
/// pass; the matches come out the same with or without sharing it — only
/// the work differs.
///
/// **One pass, not one frame.** The caches drop themselves when the
/// `Dom`'s mutation epoch moves, which only a mutation record does
/// (a tree, attribute, text or interaction change). State a backend keeps
/// behind its hooks moves no epoch: a control's user validity, its
/// default checkedness ([`ControlState`](crate::ControlState)), its
/// constraint validity ([`Dom::set_validity_hook`](crate::Dom::set_validity_hook)).
/// A `SelectorCaches` kept across frames therefore answers `:has()`
/// arguments that read them (`form:has(:user-invalid)`, `:has(:invalid)`)
/// and `:default` from before such a change. Make a new one per pass.
#[derive(Debug, Default)]
pub struct SelectorCaches {
    /// The `Dom` mutation epoch the entries were built under.
    epoch: Option<u64>,
    /// Per (parent, kind of count): each counted child's 1-based index
    /// and the number of children counted with it.
    pub(super) nth: HashMap<(NodeId, NthCount), HashMap<NodeId, (u32, u32)>>,
    /// Each element's directionality, once read (`:dir()`).
    pub(super) dir: HashMap<NodeId, Directionality>,
    /// Per [`HasKey`]: whether the element, as a `:has()` anchor, has a
    /// match — and, for a downward argument, whether the element holds a
    /// match of the argument's compounds from one on below it.
    pub(super) has: HashMap<HasKey, bool>,
    /// Per radio: whether its radio button group has no checked member
    /// (`:indeterminate`), filled for a whole group at once.
    pub(crate) radio_unchecked: HashMap<NodeId, bool>,
    /// Per form: its default button (`:default`), once found.
    pub(crate) default_buttons: HashMap<NodeId, Option<NodeId>>,
    /// Per `<table>`: HTML's table model (the column selectors), once
    /// formed.
    pub(super) tables: HashMap<NodeId, std::rc::Rc<crate::table::html::HtmlTableModel>>,
    /// The elements a `:has()` was evaluated for, in first-test order —
    /// facts for a backend to flag, not cached answers: a mutation in the
    /// middle of a pass keeps them (`sync`).
    has_anchors: Vec<NodeId>,
    has_anchor_set: HashSet<NodeId>,
    work: CacheWork,
}

/// The work a [`SelectorCaches`] did — a deterministic measure of
/// matching cost, for tests and profiling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheWork {
    /// Siblings visited while indexing sibling lists for `:nth-*()`.
    pub nth_siblings: u64,
    /// Elements visited looking for `:has()` matches.
    pub has_nodes: u64,
    /// Candidates a combinator step of a complex selector tried (the
    /// matcher's climbs to ancestors and walks to earlier siblings).
    pub chain_steps: u64,
    /// Radio button groups gathered for `:indeterminate` — one per group
    /// per pass.
    pub radio_group_walks: u64,
    /// Forms whose controls were walked for their default button
    /// (`:default`) — one per form per pass.
    pub default_button_walks: u64,
    /// Tables whose HTML table model was formed for the column selectors
    /// (`||`, `:nth-col()`) — one per table per pass.
    pub table_models: u64,
}

/// Which siblings an nth index counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NthCount {
    /// Every element sibling.
    Child,
    /// The siblings of each element's type.
    OfType,
    /// The siblings matching the `of S` list at this address, under this
    /// scoping root (`S` may read `:scope`).
    Of(usize, Option<NodeId>),
}

/// The key of a `:has()` answer: the relative selector (by address), the
/// compound it is asked from (left to right; `u32::MAX` for an anchor's
/// whole answer), the scoping root `:scope` reads, and the element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct HasKey {
    rel: usize,
    step: u32,
    scope: Option<NodeId>,
    node: NodeId,
}

impl HasKey {
    pub(super) fn new(
        rel: &crate::selectors::RelativeSelector,
        step: u32,
        scope: Option<NodeId>,
        node: NodeId,
    ) -> Self {
        Self {
            rel: std::ptr::from_ref(rel) as usize,
            step,
            scope,
            node,
        }
    }
}

impl SelectorCaches {
    /// Empty caches (no allocation until a selector needs one).
    pub fn new() -> Self {
        Self::default()
    }

    /// The work done since the caches were created.
    pub fn work(&self) -> CacheWork {
        self.work
    }

    /// The elements a `:has()` was evaluated for (its anchors) since the
    /// caches were created, in first-test order: the elements whose match
    /// can change when their subtree or later siblings do — what a
    /// backend's style invalidation watches (Selectors 4 §4.5).
    pub fn has_anchors(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.has_anchors.iter().copied()
    }

    pub(super) fn note_has_anchor(&mut self, anchor: NodeId) {
        if self.has_anchor_set.insert(anchor) {
            self.has_anchors.push(anchor);
        }
    }

    pub(super) fn count_has_node(&mut self) {
        self.work.has_nodes += 1;
    }

    pub(super) fn count_chain_step(&mut self) {
        self.work.chain_steps += 1;
    }

    /// Drop every entry built under another mutation epoch.
    pub(crate) fn sync(&mut self, epoch: u64) {
        if self.epoch != Some(epoch) {
            self.nth.clear();
            self.dir.clear();
            self.has.clear();
            self.radio_unchecked.clear();
            self.default_buttons.clear();
            self.tables.clear();
            self.epoch = Some(epoch);
        }
    }

    pub(super) fn count_table_model(&mut self) {
        self.work.table_models += 1;
    }

    pub(crate) fn count_default_button_walk(&mut self) {
        self.work.default_button_walks += 1;
    }

    pub(crate) fn count_radio_group_walk(&mut self) {
        self.work.radio_group_walks += 1;
    }

    /// Count one sibling visited while indexing.
    pub(super) fn count_nth_sibling(&mut self) {
        self.work.nth_siblings += 1;
        #[cfg(test)]
        probe::STEPS.with(|c| c.set(c.get() + 1));
    }
}

/// Test-only: sibling steps the nth indexing took on this thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        STEPS.with(|c| c.replace(0))
    }
}
