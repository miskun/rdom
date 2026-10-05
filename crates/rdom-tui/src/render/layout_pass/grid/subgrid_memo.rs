//! A layout pass's memo of subgrid sizes (CSS Grid 2 §9.5): a subgrid's
//! size on the axis it does not subgrid — a pure function of the element,
//! the axis, its area's width and what it inherits — so a chain of nested
//! subgrids is measured once a level, not once for each sizing run above
//! it. Kept in the pass's document data (`intrinsic::memo`), dropped
//! with it.
//!
//! The keys are structural: a lookup hashes its borrowed parts and
//! compares them on a hit, so it copies nothing (C7G-SUBGRID-COST; the
//! key was a `format!` of the inherited names and extents).

use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use rdom_core::NodeId;

use super::Dimension;
use super::subgrid::Inherit;

/// What a subgrid's size depends on besides its tree and styles: the
/// axis, its area's width (its padding's basis) and what it inherits.
#[derive(Debug, Clone)]
struct Key {
    id: NodeId,
    dimension: Dimension,
    area: u16,
    inherit: Inherit,
}

/// Entries bucketed by their key's hash, so a lookup by a borrowed key
/// needs no owned one.
#[derive(Debug)]
struct Table<V>(HashMap<u64, Vec<(Key, V)>>);

impl<V> Default for Table<V> {
    fn default() -> Self {
        Table(HashMap::new())
    }
}

impl<V> Table<V> {
    fn get(&self, key: KeyRef<'_>) -> Option<&V> {
        self.0
            .get(&key.hash())?
            .iter()
            .find(|(k, _)| key.is(k))
            .map(|(_, v)| v)
    }

    fn put(&mut self, key: KeyRef<'_>, value: V) {
        let hash = key.hash();
        self.0.entry(hash).or_default().push((key.owned(), value));
    }
}

/// A [`Key`] borrowed.
#[derive(Debug, Clone, Copy)]
pub(super) struct KeyRef<'a> {
    pub(super) id: NodeId,
    pub(super) dimension: Dimension,
    pub(super) area: u16,
    pub(super) inherit: &'a Inherit,
}

impl KeyRef<'_> {
    /// The bucket: a hash of the fields, borrowed.
    fn hash(self) -> u64 {
        BuildHasherDefault::<DefaultHasher>::default().hash_one((
            self.id,
            self.dimension,
            self.area,
            self.inherit,
        ))
    }

    fn is(self, k: &Key) -> bool {
        k.id == self.id
            && k.dimension == self.dimension
            && k.area == self.area
            && &k.inherit == self.inherit
    }

    fn owned(self) -> Key {
        Key {
            id: self.id,
            dimension: self.dimension,
            area: self.area,
            inherit: self.inherit.clone(),
        }
    }
}

/// The pass's subgrid memo: each subgrid's border-box min- and
/// max-content size on the axis it does not subgrid
/// (`size::measure_subgrid`).
#[derive(Debug, Default)]
pub(in crate::render::layout_pass) struct SubgridMemo(Table<(u16, u16)>);

impl SubgridMemo {
    pub(super) fn size(&self, key: KeyRef<'_>) -> Option<(u16, u16)> {
        self.0.get(key).copied()
    }

    pub(super) fn put_size(&mut self, key: KeyRef<'_>, size: (u16, u16)) {
        self.0.put(key, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_alloc::allocations_in;

    /// A hit copies nothing; a key differing in one field misses.
    #[test]
    fn a_lookup_allocates_nothing() {
        let mut dom = crate::TuiDom::new();
        let id = dom.create_element("div");
        let inherit = Inherit {
            columns: Some(super::super::subgrid::Inherited {
                tracks: 2,
                names: vec![vec!["a".into()], vec![], vec!["b".into(), "c".into()]],
                extents: Some(vec![(0, 3), (3, 7)]),
            }),
            rows: None,
        };
        let key = KeyRef {
            id,
            dimension: Dimension::Columns,
            area: 7,
            inherit: &inherit,
        };
        let mut memo = SubgridMemo::default();
        memo.put_size(key, (3, 7));
        let mut hit = None;
        assert_eq!(allocations_in(|| hit = memo.size(key)), 0);
        assert_eq!(hit, Some((3, 7)));
        let other = KeyRef { area: 8, ..key };
        assert_eq!(memo.size(other), None);
    }
}
