//! The App's media environment (Media Queries 4 / 5, CSSOM View §4.2):
//! the preferences it reports to `@media` and `matchMedia`, the lists
//! `matchMedia` watches, and the restyle a change of either asks for —
//! only when a query flips (or, for a resize, when a viewport unit is in
//! use; `style::cascade::must_restyle`).

use rdom_style::conditional::{MediaEnvironment, MediaPreferences};

use super::App;
use super::redraw::Redraw;
use crate::render::backend::Backend;
use crate::runtime::media_query::MediaQueryList;
use crate::style::CascadeExt;

impl<B: Backend> App<B> {
    /// Report `preferences` to the document's media queries (Media
    /// Queries 5 §12): `prefers-reduced-motion`, `prefers-contrast`, the
    /// pointer, the color depth, … The defaults are a terminal's with a
    /// mouse and no stated preference; a terminal has no OS setting for
    /// them, so an app that knows its user's (a `--reduced-motion` flag,
    /// a config file) says so here.
    pub fn with_media_preferences(mut self, preferences: MediaPreferences) -> Self {
        self.set_media_preferences(preferences);
        self
    }

    /// Change the reported media preferences while the app runs. The
    /// tree restyles at the next frame only if a query of its sheets
    /// flips; `matchMedia` lists report their flips then too.
    pub fn set_media_preferences(&mut self, preferences: MediaPreferences) {
        if self.dom.media_preferences() == preferences {
            return;
        }
        self.dom.set_media_preferences(preferences);
        let sheets = self.prelude.cascade_order(&self.stylesheets);
        if crate::style::cascade::must_restyle(&self.dom, &sheets, &self.prelude.registry, false) {
            self.redraw.note(Redraw::Cascade);
        }
    }

    /// The media preferences the app reports.
    pub fn media_preferences(&self) -> MediaPreferences {
        self.dom.media_preferences()
    }

    /// `window.matchMedia(query)` (CSSOM View §4.2): a live list for the
    /// media query list `query`, evaluated now against the terminal's size,
    /// the preferred color scheme and the preferences, then at every
    /// frame — its listeners called when the result flips. A list with a
    /// listener is kept by the App while it has one, so the chained
    /// `app.match_media(q).add_listener(f)` hears its flips; one without is
    /// forgotten once its handle is dropped.
    #[must_use = "a list is read through its handle, or heard through a listener added to it"]
    pub fn match_media(&mut self, query: &str) -> MediaQueryList {
        let env = self.media_environment();
        self.media_watches.watch(query, &env)
    }

    /// The environment the media queries evaluate in now: the terminal's
    /// size (as the next frame will lay out in), the document's preferred
    /// scheme and the preferences.
    fn media_environment(&self) -> MediaEnvironment {
        let mut env = crate::style::cascade::document_media(&self.dom);
        if let Ok(size) = self.terminal.backend().size() {
            env.viewport = rdom_style::calc::Viewport::new(size.width, size.height);
        }
        env
    }

    /// CSSOM View §4.2 "evaluate media queries and report changes": each
    /// `matchMedia` list whose result flipped calls its listeners, in
    /// creation order, with the document. A frame step, before the
    /// frame's style.
    pub(super) fn report_media_changes(&mut self) {
        let env = self.media_environment();
        for (listeners, event) in self.media_watches.changes(&env) {
            let mut cx =
                crate::runtime::timers::TimerCtx::new(&mut self.dom, self.scheduler.clone());
            crate::runtime::media_query::fire(&mut cx, &listeners, &event);
        }
    }
}
