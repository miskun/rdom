//! The author-stylesheet stack of an [`App`]: registration
//! ([`App::set_stylesheet`] / [`App::push_stylesheet`] /
//! [`App::remove_stylesheet`]), the opaque [`StylesheetId`] allocation,
//! the [`App::style_sheets`] accessor, applying the intents an
//! [`AppContext`](super::AppContext) queued, and the cascade
//! invalidation that a stack change implies.

use super::{App, context};
use crate::render::backend::Backend;
use crate::style::Stylesheet;

/// Opaque handle for a stylesheet registered with an [`App`]. Returned
/// by [`App::push_stylesheet`] and consumed by [`App::remove_stylesheet`].
///
/// Equality is identity-based: two ids compare equal iff they refer to
/// the same registered sheet (within the same App). Ids are never
/// reused within an App; once removed, the id becomes stale and
/// further `remove_stylesheet` calls with it are a no-op. Ids from
/// one App passed to another are also no-op on lookup miss — no panic.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct StylesheetId(pub(super) u64);

impl<B: Backend> App<B> {
    /// Replace every registered stylesheet with `sheet`. Returns the
    /// freshly-assigned [`StylesheetId`] for the new sheet, symmetric
    /// with [`Self::push_stylesheet`].
    ///
    /// Any sheets previously pushed via [`Self::push_stylesheet`] are
    /// dropped and their ids become stale (subsequent
    /// `remove_stylesheet` calls with them are no-ops). The next paint
    /// runs a full re-cascade.
    ///
    /// For incremental sheet management (adding a per-screen sheet
    /// without losing the base sheet), use [`Self::push_stylesheet`].
    pub fn set_stylesheet(&mut self, sheet: Stylesheet) -> StylesheetId {
        let id = StylesheetId(self.next_stylesheet_id);
        self.next_stylesheet_id += 1;
        self.stylesheets.clear();
        self.stylesheets.push((id, sheet));
        self.invalidate_cascade();
        id
    }

    /// Append a new author stylesheet onto the cascade stack. The
    /// returned [`StylesheetId`] can later be passed to
    /// [`Self::remove_stylesheet`] to take it back out.
    ///
    /// Within the cascade, later-pushed sheets win same-specificity
    /// contests — push order is the third tiebreaker after
    /// (specificity, source_idx). Custom-property (`var()`)
    /// definitions are merged across sheets with later-wins
    /// semantics per var name. Matches `Document.styleSheets`
    /// ordering on the web.
    ///
    /// The next paint runs a full re-cascade.
    pub fn push_stylesheet(&mut self, sheet: Stylesheet) -> StylesheetId {
        let id = StylesheetId(self.next_stylesheet_id);
        self.next_stylesheet_id += 1;
        self.stylesheets.push((id, sheet));
        self.invalidate_cascade();
        id
    }

    /// Remove a previously-pushed sheet by [`StylesheetId`]. No-op
    /// if the id is unknown (already removed, or from a different
    /// App) — never panics. When removal actually changes the
    /// stack, the next paint runs a full re-cascade.
    pub fn remove_stylesheet(&mut self, id: StylesheetId) {
        if let Some(pos) = self.stylesheets.iter().position(|(sid, _)| *sid == id) {
            self.stylesheets.remove(pos);
            self.invalidate_cascade();
        }
    }

    /// Apply the stylesheet-stack changes a handler requested through
    /// its [`AppContext`], in order. The ids the context handed out are
    /// the ones assigned here: the context started from
    /// `next_stylesheet_id` and advanced it the same way.
    pub(super) fn apply_stylesheet_intents(&mut self, intents: Vec<context::StylesheetIntent>) {
        for intent in intents {
            match intent {
                context::StylesheetIntent::Set(id, sheet) => {
                    debug_assert_eq!(id.0, self.next_stylesheet_id);
                    self.set_stylesheet(sheet);
                }
                context::StylesheetIntent::Push(id, sheet) => {
                    debug_assert_eq!(id.0, self.next_stylesheet_id);
                    self.push_stylesheet(sheet);
                }
                context::StylesheetIntent::Remove(id) => self.remove_stylesheet(id),
            }
        }
    }

    /// Drop the dirty tracker's accumulated roots and force a full
    /// re-cascade on the next paint. Used by stylesheet-mutation
    /// methods.
    ///
    /// Critically uses `take_roots` (which drains) rather than
    /// `roots_snapshot` (which peeks). If the DOM has pending dirty
    /// subtrees when this is called, leaving them in the tracker
    /// would cause the next `draw_if_dirty` to do a partial
    /// `cascade_subtrees_all` rooted at those subtrees — skipping
    /// every element outside them and leaving stale computed styles
    /// from the previous sheet stack. Draining + `needs_redraw=true`
    /// is what gets the empty-`dirty_roots` branch of `draw_if_dirty`
    /// to run the full cascade.
    fn invalidate_cascade(&mut self) {
        self.tracker.take_roots();
        self.needs_redraw = true;
    }

    /// All stylesheets registered with this App, in push order.
    /// Spec-name parity with `Document.styleSheets`.
    ///
    /// Index 0 is the sheet passed to [`Self::new`] / [`Self::with_backend`];
    /// further indices come from [`Self::push_stylesheet`] calls. The cascade
    /// merges rules across all sheets, with later sheets winning
    /// same-specificity contests.
    ///
    /// Returns a fresh `Vec` of references because storage pairs each
    /// sheet with its opaque `StylesheetId`; the allocation is
    /// negligible compared to a cascade pass.
    ///
    /// Note: stylesheets live on `App`, not on `TuiDom`. This is
    /// deliberate — a stylesheet is an App-lifecycle concept
    /// (registered at construction, mutable mid-run), whereas the
    /// `Dom` is a pure tree structure. The other three
    /// `TuiDocAccessors` methods (`element_from_point`,
    /// `elements_from_point`, `caret_position_from_point`) operate
    /// on the tree and live on `Dom`; this one operates on the
    /// runtime and lives here.
    pub fn style_sheets(&self) -> Vec<&Stylesheet> {
        self.stylesheets.iter().map(|(_, s)| s).collect()
    }
}
