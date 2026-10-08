//! The CSS Custom Highlight API 1 data model, renderer-free: a
//! [`Highlight`] is a set of ranges with a priority and a type; the
//! document's [`HighlightRegistry`] (`CSS.highlights`) maps names to
//! highlights in registration order. A renderer styles each registered
//! name's ranges with `::highlight(name)`.
//!
//! Every range a registered highlight holds is **live** (DOM §5.3): text
//! edits, insertions and removals move its boundary points as they move a
//! `Range`'s in a browser (`live`, called by the tree and text mutators).
//! A browser highlight may also hold a `StaticRange`, which no mutation
//! updates; rdom has one range type, and keeps every one live.
//!
//! **Names.** The web's members map one to one, with two Rust spellings:
//! `type` (a keyword) is `kind` — the [`kind`](Highlight::kind) accessor,
//! [`set_kind`](Highlight::set_kind) and the
//! [`with_kind`](Highlight::with_kind) builder — and the `size` of the
//! setlike `Highlight` and the maplike `HighlightRegistry` is `len()` /
//! `is_empty()` on both, as on every Rust collection.

mod live;
mod registry;

pub use registry::{HighlightRegistry, HighlightsMut};

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
/// `type` ([`kind`](Self::kind)).
///
/// Its members are read and set through methods, so a change made to a
/// registered highlight (through
/// [`HighlightRegistry::get_mut`]) is known to the registry, which
/// reports it as one `Mutation::HighlightsChanged`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Highlight {
    ranges: Vec<Range>,
    priority: i32,
    kind: HighlightType,
    /// Whether a member changed since the registry last looked.
    changed: bool,
}

impl PartialEq for Highlight {
    fn eq(&self, other: &Self) -> bool {
        (&self.ranges, self.priority, self.kind) == (&other.ranges, other.priority, other.kind)
    }
}

impl Eq for Highlight {}

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
        self.set_priority(priority);
        self
    }

    /// This highlight with `kind` as its type.
    pub fn with_kind(mut self, kind: HighlightType) -> Self {
        self.set_kind(kind);
        self
    }

    /// The paint order of overlapping highlights (§5.2): a higher
    /// priority paints above; at equal priority, the one registered
    /// later. Initial 0.
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Set the [`priority`](Self::priority).
    pub fn set_priority(&mut self, priority: i32) {
        self.changed |= self.priority != priority;
        self.priority = priority;
    }

    /// What the highlight means (§3, the web's `type`; initial
    /// `highlight`).
    pub fn kind(&self) -> HighlightType {
        self.kind
    }

    /// Set the [`kind`](Self::kind).
    pub fn set_kind(&mut self, kind: HighlightType) {
        self.changed |= self.kind != kind;
        self.kind = kind;
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
        self.changed = true;
        true
    }

    /// Remove `range`; whether it was there.
    pub fn delete(&mut self, range: &Range) -> bool {
        let before = self.ranges.len();
        self.ranges.retain(|r| r != range);
        let removed = self.ranges.len() != before;
        self.changed |= removed;
        removed
    }

    /// Whether `range` is in it.
    pub fn has(&self, range: &Range) -> bool {
        self.ranges.contains(range)
    }

    /// Remove every range.
    pub fn clear(&mut self) {
        self.changed |= !self.ranges.is_empty();
        self.ranges.clear();
    }

    /// How many ranges it holds (the web's `size`).
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Whether a member changed since the last call; clears the mark.
    fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// Every boundary point of its ranges, for the live-range updates.
    fn points_mut(&mut self) -> impl Iterator<Item = &mut Position> {
        self.ranges
            .iter_mut()
            .flat_map(|r| [&mut r.start, &mut r.end])
    }
}
