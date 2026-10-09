//! The hand-derived references, one file per tile, in `ACID.md`'s
//! numbering. Each file carries its spec citations and the derivation of
//! every cell (ground rule 1: from the spec, never from rdom's output).

use super::reference::Reference;

pub mod t01_cascade;

/// Every reference.
pub const ALL: &[&Reference] = &[&t01_cascade::REF];
