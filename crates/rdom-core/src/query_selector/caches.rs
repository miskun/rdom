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
//! Entries are keyed by node and — for `of S` — by the address of the
//! `S` selector list, so the caches are valid only while the tree and
//! the selectors they were built for are unchanged. The tree half is
//! enforced: every mutation record moves the `Dom`'s mutation epoch, and
//! caches built under another epoch are dropped before use. The
//! selectors half is the caller's: one `SelectorCaches` per pass over
//! one set of parsed selectors.

use std::collections::HashMap;

use crate::Directionality;
use crate::node_id::NodeId;

/// The caches of one selector-matching pass. Create one per pass (a
/// cascade, a query) and hand it to every
/// [`Dom::matches_list_with`](crate::Dom::matches_list_with) call of the
/// pass; the matches come out the same with or without sharing it — only
/// the work differs.
#[derive(Debug, Default)]
pub struct SelectorCaches {
    /// The `Dom` mutation epoch the entries were built under.
    epoch: Option<u64>,
    /// Per (parent, kind of count): each counted child's 1-based index
    /// and the number of children counted with it.
    pub(super) nth: HashMap<(NodeId, NthCount), HashMap<NodeId, (u32, u32)>>,
    /// Each element's directionality, once read (`:dir()`).
    pub(super) dir: HashMap<NodeId, Directionality>,
    work: CacheWork,
}

/// The work a [`SelectorCaches`] did — a deterministic measure of
/// matching cost, for tests and profiling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheWork {
    /// Siblings visited while indexing sibling lists for `:nth-*()`.
    pub nth_siblings: u64,
}

/// Which siblings an nth index counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NthCount {
    /// Every element sibling.
    Child,
    /// The siblings of each element's type.
    OfType,
    /// The siblings matching the `of S` list at this address.
    Of(usize),
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

    /// Drop every entry built under another mutation epoch.
    pub(super) fn sync(&mut self, epoch: u64) {
        if self.epoch != Some(epoch) {
            self.nth.clear();
            self.dir.clear();
            self.epoch = Some(epoch);
        }
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
