//! The author-stylesheet stack of an [`App`]: registration
//! ([`App::set_stylesheet`] / [`App::push_stylesheet`] /
//! [`App::remove_stylesheet`]), the opaque [`StylesheetId`] and its one
//! allocator (shared with every [`AppContext`](super::AppContext)),
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
pub struct StylesheetId(u64);

/// Hands out [`StylesheetId`]s, monotonically, never reusing one. An
/// [`App`] owns exactly one; its [`AppContext`](super::AppContext)s
/// borrow it, so every id — from a direct call or a handler's intent —
/// comes from the same counter. u64 is overkill for in-process
/// lifetimes — chosen for simplicity.
#[derive(Debug, Default)]
pub(super) struct StylesheetIdAllocator {
    next: u64,
}

impl StylesheetIdAllocator {
    pub(super) fn allocate(&mut self) -> StylesheetId {
        let id = StylesheetId(self.next);
        self.next += 1;
        id
    }
}

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
        let id = self.stylesheet_ids.allocate();
        self.replace_stylesheets(id, sheet);
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
        let id = self.stylesheet_ids.allocate();
        self.append_stylesheet(id, sheet);
        id
    }

    /// Register `sheet` under the already-allocated `id` as the only
    /// sheet.
    fn replace_stylesheets(&mut self, id: StylesheetId, sheet: Stylesheet) {
        self.stylesheets.clear();
        self.append_stylesheet(id, sheet);
    }

    /// Register `sheet` under the already-allocated `id` at the end of
    /// the stack.
    fn append_stylesheet(&mut self, id: StylesheetId, sheet: Stylesheet) {
        debug_assert!(
            self.stylesheets.iter().all(|(sid, _)| *sid != id),
            "a StylesheetId is registered at most once"
        );
        self.stylesheets.push((id, sheet));
        self.invalidate_cascade();
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
    /// its [`AppContext`], in order. Each sheet is registered under the
    /// id the context allocated (from this App's allocator) and handed
    /// back to the handler.
    pub(super) fn apply_stylesheet_intents(&mut self, intents: Vec<context::StylesheetIntent>) {
        for intent in intents {
            match intent {
                context::StylesheetIntent::Set(id, sheet) => self.replace_stylesheets(id, sheet),
                context::StylesheetIntent::Push(id, sheet) => self.append_stylesheet(id, sheet),
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
    /// from the previous sheet stack. Draining + `Redraw::Cascade`
    /// is what gets the empty-`dirty_roots` branch of `draw_if_dirty`
    /// to run the full cascade.
    pub(super) fn invalidate_cascade(&mut self) {
        self.prelude.sheets_changed(
            &mut self.dom,
            &self.tracker,
            &self.stylesheets,
            &mut self.redraw,
        );
    }

    /// All stylesheets registered with this App, in push order.
    /// Spec-name parity with `Document.styleSheets`. The document's
    /// `<style>` element sheets are not in this list: the App keeps them
    /// itself, live, and cascades them before these
    /// (`cssom::style_elements`).
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

    /// Resolve the `@import` rules of the document's `<style>` elements
    /// through `loader` (CSS Cascade 5 §3) — rdom has no network or
    /// filesystem policy of its own, so the host decides what a URL
    /// means. Every `<style>` sheet is re-parsed and the next paint
    /// re-cascades. Sheets built in Rust import through
    /// [`rdom_css::parse_with_loader`] instead. Example: see
    /// [`register_property`](Self::register_property).
    pub fn set_import_loader(&mut self, loader: impl crate::ImportLoader + 'static) {
        self.prelude
            .style_elements
            .set_loader(Some(std::rc::Rc::new(loader)));
        self.invalidate_cascade();
    }

    /// `CSS.registerProperty` (CSS Properties and Values API 1 §3):
    /// register a custom property for every sheet of this App. It wins
    /// over an `@property` for the same name. A name registered here
    /// once cannot be registered again
    /// ([`RegisterPropertyError::AlreadyRegistered`](crate::RegisterPropertyError::AlreadyRegistered),
    /// the web API's `InvalidModificationError`); the other ways a
    /// registration fails are caught when it is built
    /// ([`PropertyRegistration::new`](crate::PropertyRegistration::new)).
    /// The next paint re-cascades.
    ///
    /// With [`set_import_loader`](Self::set_import_loader), using
    /// `rdom_tui` paths only:
    ///
    /// ```
    /// use rdom_tui::{
    ///     App, Color, PropertyRegistration, Stylesheet, Terminal, TestBackend, TuiDom,
    ///     TuiNodeExt,
    /// };
    ///
    /// let mut dom: TuiDom = TuiDom::new();
    /// let root = dom.root();
    /// let style = dom.create_element("style");
    /// let css = dom.create_text_node("@import 'theme.css'; p { color: var(--accent) }");
    /// dom.append_child(style, css).unwrap();
    /// dom.append_child(root, style).unwrap();
    /// let (a, b) = (dom.create_element("p"), dom.create_element("p"));
    /// dom.set_attribute(a, "class", "a").unwrap();
    /// dom.set_attribute(b, "class", "b").unwrap();
    /// dom.append_child(root, a).unwrap();
    /// dom.append_child(root, b).unwrap();
    ///
    /// let terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
    /// let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    /// // Any `Fn(&str) -> Result<String, String>` is an `ImportLoader`.
    /// app.set_import_loader(|url: &str| match url {
    ///     "theme.css" => Ok(".a { --accent: red } .b { --accent: 3 }".to_string()),
    ///     other => Err(format!("no {other}")),
    /// });
    /// let accent = PropertyRegistration::new("--accent", "<color>", true, Some("blue")).unwrap();
    /// app.register_property(accent).unwrap();
    /// app.advance(0).unwrap();
    /// let fg = |id| app.dom().node(id).computed().unwrap().fg;
    /// // The imported theme applies...
    /// assert_eq!(fg(a), Color::Rgb(255, 0, 0));
    /// // ...and `3` is not a `<color>`: invalid at computed-value time,
    /// // so the registered initial value applies.
    /// assert_eq!(fg(b), Color::Rgb(0, 0, 255));
    /// ```
    pub fn register_property(
        &mut self,
        registration: crate::PropertyRegistration,
    ) -> Result<(), crate::RegisterPropertyError> {
        let registered = &mut self.prelude.registrations;
        if registered
            .registered_properties()
            .iter()
            .any(|r| r.name == registration.name)
        {
            return Err(crate::RegisterPropertyError::AlreadyRegistered(
                registration.name,
            ));
        }
        registered.register_property(registration);
        self.invalidate_cascade();
        Ok(())
    }

    /// The parse warnings of the document's `<style>` elements, in tree
    /// order, as of the last frame (`cssom::style_elements`) — what
    /// [`extend_from_style_tags`](crate::extend_from_style_tags) returns
    /// for a snapshot.
    pub fn style_element_warnings(&self) -> Vec<&rdom_css::Warning> {
        self.prelude.style_elements.warnings().collect()
    }
}
