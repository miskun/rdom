//! The App's color-scheme options: the document's preferred scheme
//! (CSS Color Adjust 1 §2.1), asked of the terminal at startup unless
//! the app sets it (`runtime::color_scheme`).

use rdom_style::color::ColorScheme;

use super::App;
use super::redraw::Redraw;
use crate::render::backend::Backend;
use crate::style::CascadeExt;

impl<B: Backend> App<B> {
    /// Use `scheme` as the document's preferred color scheme — what
    /// `light-dark()` picks by for an element with `color-scheme:
    /// normal` — instead of asking the terminal for its background at
    /// startup: `run` then skips the query and its wait (up to 200 ms,
    /// see [`App::run`]), and [`Self::detected_background`] stays
    /// `None`.
    pub fn with_color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.set_color_scheme(scheme);
        self
    }

    /// Change the document's preferred color scheme (an app that learns
    /// of a theme change, say). The whole tree cascades again at the
    /// next frame; the terminal is no longer asked at startup.
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.color_scheme_explicit = true;
        self.dom.set_color_scheme(scheme);
        self.redraw.note(Redraw::Cascade);
    }

    /// The document's preferred color scheme: set by the app, read
    /// from the terminal at startup, or dark.
    pub fn color_scheme(&self) -> ColorScheme {
        self.dom.color_scheme()
    }

    /// The background the terminal reported to the startup query
    /// (OSC 11), from which the preferred scheme was taken. `None` when
    /// the terminal did not answer, the query was not made (the app set
    /// the scheme, `run` has not started, stdout is not a terminal, a
    /// non-Unix platform) — so "the terminal said dark" is
    /// `Some(dark color)` and "it said nothing" is `None`, though both
    /// prefer dark.
    pub fn detected_background(&self) -> Option<crate::Color> {
        self.detected_background
    }

    /// Record the startup query's answer and prefer the scheme it calls
    /// for; no answer leaves the scheme as it is.
    pub(super) fn apply_detected_background(&mut self, background: Option<crate::Color>) {
        self.detected_background = background;
        if let Some(bg) = background {
            self.dom.set_color_scheme(ColorScheme::for_background(bg));
            self.redraw.note(Redraw::Cascade);
        }
    }

    /// At startup, unless the app set a scheme: ask the terminal for
    /// its background (OSC 11) and prefer the scheme it calls for.
    pub(super) fn detect_color_scheme(&mut self) {
        if self.color_scheme_explicit {
            return;
        }
        let background = crate::runtime::color_scheme::query_terminal_background(
            crate::runtime::color_scheme::QUERY_TIMEOUT,
        );
        self.apply_detected_background(background);
    }
}

#[cfg(test)]
mod tests {
    use rdom_style::color::ColorScheme;

    use crate::TuiDom;
    use crate::render::{Terminal, TestBackend};
    use crate::{App, Color, TuiNodeExt};

    /// An App over `<div>` styled by `css`, one frame drawn.
    fn app(css: &str) -> (App<TestBackend>, rdom_core::NodeId) {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        let root = dom.root();
        dom.append_child(root, div).unwrap();
        let sheet = rdom_css::parse(css).stylesheet;
        let terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
        (App::with_backend(dom, sheet, terminal).unwrap(), div)
    }

    /// CSS Color 5 §5: `light-dark()` follows the scheme the App is
    /// given, and a change restyles the tree.
    #[test]
    fn the_app_sets_the_preferred_color_scheme() {
        let (app, div) = app("div { color: light-dark(red, blue) }");
        let mut app = app.with_color_scheme(ColorScheme::Light);
        assert_eq!(app.color_scheme(), ColorScheme::Light);
        app.draw_if_dirty().unwrap();
        let fg = |app: &App<TestBackend>| app.dom().node(div).computed().unwrap().fg;
        assert_eq!(fg(&app), Color::Rgb(255, 0, 0));
        app.set_color_scheme(ColorScheme::Dark);
        app.draw_if_dirty().unwrap();
        assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
    }

    /// The startup query's answer is kept: a background the terminal
    /// reported is `Some` — and picks the scheme — while no answer leaves
    /// `None` and the default dark scheme, so an app can tell "the
    /// terminal said dark" from "the terminal said nothing".
    #[test]
    fn the_detected_background_tells_an_answer_from_none() {
        let (mut app, _) = app("");
        assert_eq!(app.detected_background(), None, "not asked yet");
        app.apply_detected_background(None);
        assert_eq!(app.detected_background(), None);
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
        app.apply_detected_background(Some(Color::Rgb(0, 0, 0)));
        assert_eq!(app.detected_background(), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
        app.apply_detected_background(Some(Color::Rgb(250, 250, 240)));
        assert_eq!(app.color_scheme(), ColorScheme::Light);
    }

    /// Dark until set — what a terminal that does not answer gets.
    #[test]
    fn the_default_scheme_is_dark() {
        let (app, _) = app("");
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
    }
}
