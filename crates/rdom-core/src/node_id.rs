//! `NodeId` — stable handle into the arena.
//!
//! `NonZeroU32` so `Option<NodeId>` packs into 4 bytes. Index = `id - 1`.
//! The `NodeId` itself is plain `Copy`; mixing ids across different `Dom`s
//! is a bug (same as mixing `slab::Key`s).

use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId(NonZeroU32);

impl NodeId {
    /// Internal arena index (id - 1).
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.0.get() as usize - 1
    }

    /// Construct from an arena index. Panics on overflow (should be
    /// impossible in practice — 4 billion nodes would OOM the process
    /// long before the counter rolls).
    #[inline]
    pub(crate) fn from_index(i: usize) -> Self {
        NodeId(
            NonZeroU32::new((i + 1) as u32).expect("arena overflow: more than u32::MAX - 1 nodes"),
        )
    }

    /// Raw numeric form for debugging / Display.
    pub fn as_u32(self) -> u32 {
        self.0.get()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip() {
        for i in [0, 1, 2, 7, 99, 1000, 1_000_000] {
            let id = NodeId::from_index(i);
            assert_eq!(id.index(), i);
        }
    }

    #[test]
    fn option_node_id_is_four_bytes() {
        assert_eq!(std::mem::size_of::<Option<NodeId>>(), 4);
    }

    #[test]
    fn display_shows_decimal() {
        let id = NodeId::from_index(41);
        assert_eq!(format!("{id}"), "#42");
    }
}
