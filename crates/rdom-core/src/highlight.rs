//! The CSS Custom Highlight API 1 data model, renderer-free: a
//! [`Highlight`] is a set of ranges with a priority and a type; the
//! document's [`HighlightRegistry`] (`CSS.highlights`) maps names to
//! highlights in registration order. A renderer styles each registered
//! name's ranges with `::highlight(name)`.
//!
//! Every range a registered highlight holds is **live** (DOM §5.3): text
//! edits, insertions and removals move its boundary points as they move a
//! `Range`'s in a browser ([`HighlightRegistry::replace_data`],
//! [`HighlightRegistry::inserted`], [`HighlightRegistry::removing`],
//! called by the tree and text mutators). A browser highlight may also
//! hold a `StaticRange`, which no mutation updates; rdom has one range
//! type, and keeps every one live.

use crate::node_id::NodeId;
use crate::selection::{Position, Range};

/// A highlight's `type` (CSS Custom Highlight API 1 §3): what the
/// highlighted ranges mean to assistive technology. It changes nothing
/// painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum HighlightType {
    /// `highlight` (the initial value).
    #[default]
    Highlight,
    /// `spelling-error`.
    SpellingError,
    /// `grammar-error`.
    GrammarError,
}

/// A set of ranges styled together (CSS Custom Highlight API 1 §3,
/// `Highlight`): no range twice, in insertion order, with a `priority`
/// that orders overlapping highlights (§5.2; higher paints above) and a
/// `type`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Highlight {
    ranges: Vec<Range>,
    /// The paint order of overlapping highlights (§5.2): a higher
    /// priority paints above; at equal priority, the one registered
    /// later. Initial 0.
    pub priority: i32,
    /// What the highlight means (§3; initial `highlight`).
    pub kind: HighlightType,
}

impl Highlight {
    /// A highlight of `ranges` (a repeated range is kept once), priority
    /// 0, type `highlight`.
    pub fn new(ranges: impl IntoIterator<Item = Range>) -> Self {
        let mut out = Self::default();
        for r in ranges {
            out.add(r);
        }
        out
    }

    /// This highlight with `priority`.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// This highlight with `kind` as its type.
    pub fn with_type(mut self, kind: HighlightType) -> Self {
        self.kind = kind;
        self
    }

    /// Its ranges, in insertion order.
    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    /// Add `range`; `false` when it is already there (a set).
    pub fn add(&mut self, range: Range) -> bool {
        if self.ranges.contains(&range) {
            return false;
        }
        self.ranges.push(range);
        true
    }

    /// Remove `range`; whether it was there.
    pub fn delete(&mut self, range: &Range) -> bool {
        let before = self.ranges.len();
        self.ranges.retain(|r| r != range);
        self.ranges.len() != before
    }

    /// Whether `range` is in it.
    pub fn has(&self, range: &Range) -> bool {
        self.ranges.contains(range)
    }

    /// Remove every range.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// How many ranges it holds.
    pub fn size(&self) -> usize {
        self.ranges.len()
    }

    /// Every boundary point of its ranges, for the live-range updates.
    fn points_mut(&mut self) -> impl Iterator<Item = &mut Position> {
        self.ranges
            .iter_mut()
            .flat_map(|r| [&mut r.start, &mut r.end])
    }
}

/// The document's highlights by name (CSS Custom Highlight API 1 §4,
/// `HighlightRegistry` — `CSS.highlights`), in registration order: the
/// tie-break of equal priorities (§5.2). Read it through
/// [`Dom::highlights`](crate::Dom::highlights), change it through
/// [`Dom::highlights_mut`](crate::Dom::highlights_mut).
#[derive(Debug, Clone, Default)]
pub struct HighlightRegistry {
    entries: Vec<(String, Highlight)>,
    /// Moves whenever a highlight or a range may have changed.
    generation: u64,
}

impl PartialEq for HighlightRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for HighlightRegistry {}

impl HighlightRegistry {
    /// Register `highlight` as `name` (a map's `set`): a registered name
    /// keeps its place and takes the new highlight, which is returned in
    /// the old one's stead.
    pub fn set(&mut self, name: impl Into<String>, highlight: Highlight) -> Option<Highlight> {
        let name = name.into();
        match self.entries.iter_mut().find(|(n, _)| *n == name) {
            Some((_, h)) => Some(std::mem::replace(h, highlight)),
            None => {
                self.entries.push((name, highlight));
                None
            }
        }
    }

    /// The highlight registered as `name`.
    pub fn get(&self, name: &str) -> Option<&Highlight> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, h)| h)
    }

    /// The highlight registered as `name`, to change.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Highlight> {
        self.entries
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h)
    }

    /// Whether `name` is registered.
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Unregister `name`; whether it was registered.
    pub fn delete(&mut self, name: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(n, _)| n != name);
        self.entries.len() != before
    }

    /// Unregister every highlight.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many highlights are registered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether none is.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The registered highlights with their names, in registration order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Highlight)> {
        self.entries.iter().map(|(n, h)| (n.as_str(), h))
    }

    /// A number that moves whenever a registered highlight or one of its
    /// ranges may have changed — at every [`Dom::highlights_mut`](crate::Dom::highlights_mut)
    /// and every DOM mutation that moves a live range: a renderer that
    /// indexes the ranges rebuilds its index when it moves.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Every boundary point of every registered range.
    fn points(&self) -> impl Iterator<Item = &Position> {
        self.entries
            .iter()
            .flat_map(|(_, h)| &h.ranges)
            .flat_map(|r| [&r.start, &r.end])
    }

    /// Whether a boundary point of a registered range is in `node`.
    fn has_point_in(&self, node: NodeId) -> bool {
        self.points().any(|p| p.node == node)
    }

    /// Every boundary point of every registered range.
    fn points_mut(&mut self) -> impl Iterator<Item = &mut Position> {
        self.entries.iter_mut().flat_map(|(_, h)| h.points_mut())
    }

    /// DOM §4.10 "replace data" steps 8–11, for `count` bytes at `offset`
    /// of `node`'s data replaced by `len` bytes: a boundary inside the
    /// replaced bytes moves to `offset`, one after them by `len - count`.
    pub(crate) fn replace_data(&mut self, node: NodeId, offset: usize, count: usize, len: usize) {
        self.generation = self.generation.wrapping_add(1);
        for p in self.points_mut().filter(|p| p.node == node) {
            if p.offset > offset && p.offset <= offset + count {
                p.offset = offset;
            } else if p.offset > offset + count {
                p.offset = p.offset + len - count;
            }
        }
    }

    /// DOM §4.2.3 "insert" step 6, for one node inserted at `index` of
    /// `parent`'s children: a boundary in `parent` past `index` moves right.
    pub(crate) fn inserted(&mut self, parent: NodeId, index: usize) {
        self.generation = self.generation.wrapping_add(1);
        for p in self.points_mut().filter(|p| p.node == parent) {
            if p.offset > index {
                p.offset += 1;
            }
        }
    }

    /// DOM §4.2.3 "remove" steps 4–7, for the child at `index` of `parent`
    /// about to be removed (`inside` tells which nodes are in its
    /// subtree): a boundary inside it moves to `(parent, index)`, one in
    /// `parent` past `index` moves left.
    pub(crate) fn removing(
        &mut self,
        parent: NodeId,
        index: usize,
        inside: impl Fn(NodeId) -> bool,
    ) {
        self.generation = self.generation.wrapping_add(1);
        for p in self.points_mut() {
            if inside(p.node) {
                *p = Position::new(parent, index);
            } else if p.node == parent && p.offset > index {
                p.offset -= 1;
            }
        }
    }
}

impl<Ext: 'static> crate::Dom<Ext> {
    /// The document's highlight registry (CSS Custom Highlight API 1 §4,
    /// `CSS.highlights`).
    pub fn highlights(&self) -> &HighlightRegistry {
        &self.highlights
    }

    /// The registry, to register, change or remove highlights. Fires
    /// `Mutation::HighlightsChanged` first, so a renderer observing
    /// mutations repaints the highlights.
    pub fn highlights_mut(&mut self) -> &mut HighlightRegistry {
        self.fire_mutation(crate::Mutation::HighlightsChanged);
        self.highlights.generation = self.highlights.generation.wrapping_add(1);
        &mut self.highlights
    }

    /// The index of `id` among its parent's children.
    pub(crate) fn child_index(&self, id: NodeId) -> usize {
        let mut index = 0;
        let mut cur = self.get_node(id).and_then(|n| n.prev_sibling);
        while let Some(prev) = cur {
            #[cfg(test)]
            crate::highlight_tests::INDEX_HOPS.with(|c| c.set(c.get() + 1));
            index += 1;
            cur = self.get_node(prev).and_then(|n| n.prev_sibling);
        }
        index
    }

    /// The live ranges after `child` was inserted under `parent` (DOM
    /// §4.2.3 "insert" step 6: a boundary in `parent` past the insertion
    /// index moves right). Free with no highlight registered, and for an
    /// append — the index is the old child count, which no boundary
    /// offset exceeds; elsewhere the index is walked only when a boundary
    /// sits in `parent`.
    pub(crate) fn highlights_inserted(&mut self, parent: NodeId, child: NodeId) {
        if self.highlights.is_empty()
            || self.get_node(child).and_then(|n| n.next_sibling).is_none()
            || !self.highlights.has_point_in(parent)
        {
            return;
        }
        let index = self.child_index(child);
        self.highlights.inserted(parent, index);
    }

    /// The live ranges before `id` is removed from its parent (DOM
    /// §4.2.3 "remove" steps 4–7). Free with no highlight registered.
    pub(crate) fn highlights_removing(&mut self, id: NodeId) {
        if self.highlights.is_empty() {
            return;
        }
        let Some(parent) = self.get_node(id).and_then(|n| n.parent) else {
            return;
        };
        // The index is walked only when a boundary moves: one in `parent`
        // or inside the removed subtree.
        let moves = |p: &Position| p.node == parent || self.is_ancestor(id, p.node);
        if !self.highlights.points().any(moves) {
            return;
        }
        let index = self.child_index(id);
        let mut registry = std::mem::take(&mut self.highlights);
        registry.removing(parent, index, |n| self.is_ancestor(id, n));
        self.highlights = registry;
    }

    /// The live ranges after `count` bytes at `offset` of the text `node`
    /// were replaced by `len` bytes (DOM §4.10 "replace data").
    pub(crate) fn highlights_replace_data(
        &mut self,
        node: NodeId,
        (offset, count, len): (usize, usize, usize),
    ) {
        if !self.highlights.is_empty() {
            self.highlights.replace_data(node, offset, count, len);
        }
    }
}
