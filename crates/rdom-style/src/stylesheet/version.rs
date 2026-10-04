//! A stylesheet's content version: what a cache keyed by a sheet set
//! compares (the cascade's registered-property registry for the
//! stateless `CascadeExt` forms).

use std::sync::atomic::{AtomicU64, Ordering};

/// A process-unique stamp, renewed by every mutation of its sheet. A
/// clone is a new sheet and gets its own, so two sheets never share
/// one: equal versions mean the same sheet, unchanged.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Version(u64);

impl Version {
    pub(super) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Version(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub(super) fn get(&self) -> u64 {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::next()
    }
}

impl Clone for Version {
    fn clone(&self) -> Self {
        Self::next()
    }
}
