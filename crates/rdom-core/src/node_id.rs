//! `NodeId` — stable handle into the arena.
//!
//! A `NodeId` is an arena slot index plus a **generation**. Freed slots
//! are recycled (LIFO, cache-friendly), and every recycle bumps the
//! slot's generation, so a handle to a dropped node can never resolve
//! to the node that later reuses its slot: `Dom::contains` says `false`,
//! `node_or_err` returns `InvalidNode`, and every mutation path refuses
//! it. This is the "never hand out a stale `NodeId` as a live one"
//! invariant; before generations existed a cached id silently aliased
//! the new occupant.
//!
//! The generation is `NonZeroU32` so `Option<NodeId>` packs into 8 bytes.
//! The `NodeId` itself is plain `Copy`; mixing ids across different
//! `Dom`s is a bug (same as mixing `slab::Key`s).

use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId {
    index: u32,
    generation: NonZeroU32,
}

impl NodeId {
    /// Internal arena index.
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.index as usize
    }

    /// Construct from an arena index + slot generation. Panics on index
    /// overflow (should be impossible in practice — 4 billion nodes
    /// would OOM the process long before the counter rolls).
    #[inline]
    pub(crate) fn from_parts(index: usize, generation: NonZeroU32) -> Self {
        let index = u32::try_from(index).expect("arena overflow: more than u32::MAX nodes");
        NodeId { index, generation }
    }

    /// Generation as stored (for the arena's liveness check).
    #[inline]
    pub(crate) fn generation_raw(self) -> NonZeroU32 {
        self.generation
    }

    /// The slot's generation when this id was issued. Starts at 1 and
    /// increments each time the slot is freed and reused.
    #[inline]
    pub fn generation(self) -> u32 {
        self.generation.get()
    }

    /// Raw slot number (`index + 1`) for debugging / Display. Two ids
    /// with the same `as_u32()` but different `generation()` refer to
    /// different nodes that occupied the same slot at different times.
    pub fn as_u32(self) -> u32 {
        self.index + 1
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.index + 1)?;
        if self.generation.get() > 1 {
            write!(f, "@{}", self.generation.get())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const G1: NonZeroU32 = NonZeroU32::MIN;

    #[test]
    fn index_round_trip() {
        for i in [0, 1, 2, 7, 99, 1000, 1_000_000] {
            let id = NodeId::from_parts(i, G1);
            assert_eq!(id.index(), i);
            assert_eq!(id.generation(), 1);
        }
    }

    /// Index + generation: 8 bytes, and `Option<NodeId>` still packs
    /// into the same 8 via the `NonZeroU32` niche.
    #[test]
    fn option_node_id_is_eight_bytes() {
        assert_eq!(std::mem::size_of::<NodeId>(), 8);
        assert_eq!(std::mem::size_of::<Option<NodeId>>(), 8);
    }

    #[test]
    fn same_slot_different_generation_is_a_different_id() {
        let a = NodeId::from_parts(5, G1);
        let b = NodeId::from_parts(5, NonZeroU32::new(2).unwrap());
        assert_ne!(a, b);
        assert_eq!(a.index(), b.index());
    }

    /// First-generation ids print as before (`#slot`); a recycled slot
    /// shows its generation so debug output can tell the two apart.
    #[test]
    fn display_shows_slot_and_generation_after_reuse() {
        assert_eq!(format!("{}", NodeId::from_parts(41, G1)), "#42");
        assert_eq!(
            format!("{}", NodeId::from_parts(41, NonZeroU32::new(3).unwrap())),
            "#42@3"
        );
    }
}
