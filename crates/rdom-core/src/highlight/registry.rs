//! The document's [`HighlightRegistry`] and [`HighlightsMut`], the guard
//! through which it changes: a change is reported once, after it is
//! made, as `Mutation::HighlightsChanged`.

use std::ops::{Deref, DerefMut};

use super::Highlight;
use crate::node_id::NodeId;
use crate::selection::Position;

/// The document's highlights by name (CSS Custom Highlight API 1 §4,
/// `HighlightRegistry` — `CSS.highlights`), in registration order: the
/// tie-break of equal priorities (§5.2). Read it through
/// [`Dom::highlights`](crate::Dom::highlights), change it through
/// [`Dom::highlights_mut`](crate::Dom::highlights_mut).
#[derive(Debug, Clone, Default)]
pub struct HighlightRegistry {
    entries: Vec<(String, Highlight)>,
    /// Moves whenever a highlight or a range changed.
    generation: u64,
    /// Whether an entry was set or removed since the last
    /// [`take_changed`](Self::take_changed).
    changed: bool,
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
        self.changed = true;
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

    /// The highlight registered as `name`, to change: what is changed
    /// through it is reported when the [`HighlightsMut`] goes.
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
        let removed = self.entries.len() != before;
        self.changed |= removed;
        removed
    }

    /// Unregister every highlight.
    pub fn clear(&mut self) {
        self.changed |= !self.entries.is_empty();
        self.entries.clear();
    }

    /// How many highlights are registered (the web's `size`).
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
    /// ranges changed — a change made through
    /// [`Dom::highlights_mut`](crate::Dom::highlights_mut), and every DOM
    /// mutation that may move a live range: a renderer that indexes the
    /// ranges rebuilds its index when it moves.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether the registry or a registered highlight changed since the
    /// last call; clears every mark.
    fn take_changed(&mut self) -> bool {
        let mut changed = std::mem::take(&mut self.changed);
        for (_, h) in &mut self.entries {
            changed |= h.take_changed();
        }
        changed
    }

    /// Every boundary point of every registered range.
    pub(super) fn points(&self) -> impl Iterator<Item = &Position> {
        self.entries
            .iter()
            .flat_map(|(_, h)| &h.ranges)
            .flat_map(|r| [&r.start, &r.end])
    }

    /// Whether a boundary point of a registered range is in `node`.
    pub(super) fn has_point_in(&self, node: NodeId) -> bool {
        self.points().any(|p| p.node == node)
    }

    /// Every boundary point of every registered range.
    fn points_mut(&mut self) -> impl Iterator<Item = &mut Position> {
        self.entries.iter_mut().flat_map(|(_, h)| h.points_mut())
    }

    /// Move the generation: a highlight or a range changed.
    pub(crate) fn bump(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// DOM §4.10 "replace data" steps 8–11, for `count` bytes at `offset`
    /// of `node`'s data replaced by `len` bytes: a boundary inside the
    /// replaced bytes moves to `offset`, one after them by `len - count`.
    pub(crate) fn replace_data(&mut self, node: NodeId, offset: usize, count: usize, len: usize) {
        self.bump();
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
        self.bump();
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
        mut inside: impl FnMut(NodeId) -> bool,
    ) {
        self.bump();
        for p in self.points_mut() {
            if inside(p.node) {
                *p = Position::new(parent, index);
            } else if p.node == parent && p.offset > index {
                p.offset -= 1;
            }
        }
    }
}

/// The registry taken for change ([`Dom::highlights_mut`](crate::Dom::highlights_mut)):
/// it reads and changes as a `&mut HighlightRegistry` (`Deref` /
/// `DerefMut`), and when it goes — at the end of the statement for
/// `dom.highlights_mut().set(…)` — it reports what changed: if a highlight
/// was set or removed, or a registered one changed through `get_mut`, the
/// registry's [`generation`](HighlightRegistry::generation) moves and one
/// `Mutation::HighlightsChanged` fires, after the change (a mutation
/// record follows its mutation, DOM §4.3), so an observer reads the new
/// registry. A read, a `get_mut` miss, a highlight left as it was, a
/// `delete` of an unregistered name and a `clear` of an empty registry
/// report nothing.
pub struct HighlightsMut<'a, Ext: 'static> {
    dom: &'a mut crate::Dom<Ext>,
}

impl<'a, Ext: 'static> HighlightsMut<'a, Ext> {
    pub(super) fn new(dom: &'a mut crate::Dom<Ext>) -> Self {
        Self { dom }
    }
}

impl<Ext: 'static> Deref for HighlightsMut<'_, Ext> {
    type Target = HighlightRegistry;

    fn deref(&self) -> &HighlightRegistry {
        &self.dom.highlights
    }
}

impl<Ext: 'static> DerefMut for HighlightsMut<'_, Ext> {
    fn deref_mut(&mut self) -> &mut HighlightRegistry {
        &mut self.dom.highlights
    }
}

impl<Ext: 'static> Drop for HighlightsMut<'_, Ext> {
    fn drop(&mut self) {
        if !self.dom.highlights.take_changed() {
            return;
        }
        self.dom.highlights.bump();
        // Unwinding from a panic while the guard was held: the change is
        // recorded (the generation moved), but no observer runs.
        if !std::thread::panicking() {
            self.dom.fire_mutation(crate::Mutation::HighlightsChanged);
        }
    }
}
