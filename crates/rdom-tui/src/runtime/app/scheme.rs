//! The App's color-scheme options: the document's preferred scheme
//! (CSS Color Adjust 1 §2.1), asked of the terminal at startup and
//! followed as the terminal reports theme changes (DEC mode 2031),
//! unless the app sets it (`runtime::color_scheme`).

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

    /// Change the document's preferred color scheme. The whole tree
    /// cascades again at the next frame; from here on the scheme is the
    /// app's: the terminal is not asked at startup, and its theme-change
    /// reports (mode 2031) and late background replies are ignored.
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
    /// (OSC 11) — or later, when its reply came after the wait — from
    /// which the preferred scheme was taken (a mode 2031 theme report
    /// changes the scheme, not this). `None` when
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
    /// its background (OSC 11) and prefer the scheme it calls for. The
    /// replies are read through `input`, which keeps the keys typed
    /// meanwhile.
    pub(super) fn detect_color_scheme(&mut self, input: &mut crate::runtime::input::InputReader) {
        if self.color_scheme_explicit {
            return;
        }
        let background = crate::runtime::color_scheme::query_terminal_background(
            input,
            crate::runtime::color_scheme::QUERY_TIMEOUT,
        );
        self.apply_detected_background(background);
    }

    /// The terminal reported its background outside the startup wait (a
    /// late OSC 11 reply): kept, and its scheme preferred — unless the
    /// app set the scheme, which the terminal does not override.
    pub(super) fn note_terminal_background(&mut self, background: crate::Color) {
        if !self.color_scheme_explicit {
            self.apply_detected_background(Some(background));
        }
    }

    /// The terminal's theme changed (a DEC mode 2031 report): prefer its
    /// scheme and restyle — unless the app set the scheme.
    pub(super) fn note_terminal_scheme(&mut self, scheme: ColorScheme) {
        if self.color_scheme_explicit || self.dom.color_scheme() == scheme {
            return;
        }
        self.dom.set_color_scheme(scheme);
        self.redraw.note(Redraw::Cascade);
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

    /// The App's input, parsed from `bytes` as the terminal sent them.
    fn send(app: &mut App<TestBackend>, bytes: &[u8]) {
        let mut parser = crate::runtime::input::Parser::default();
        parser.feed(bytes);
        while let Some(input) = parser.next() {
            app.handle_input(input);
        }
    }

    /// DEC mode 2031: the terminal's theme report (`CSI ? 997 ; 2 n`
    /// light, `; 1 n` dark) changes the preferred scheme, and the tree
    /// restyles — `light-dark()` follows the terminal's theme.
    #[test]
    fn a_mode_2031_report_changes_the_scheme() {
        let (mut app, div) = app("div { color: light-dark(red, blue) }");
        app.draw_if_dirty().unwrap();
        let fg = |app: &App<TestBackend>| app.dom().node(div).computed().unwrap().fg;
        assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
        send(&mut app, b"\x1b[?997;2n");
        assert_eq!(app.color_scheme(), ColorScheme::Light);
        app.draw_if_dirty().unwrap();
        assert_eq!(fg(&app), Color::Rgb(255, 0, 0));
        send(&mut app, b"\x1b[?997;1n");
        app.draw_if_dirty().unwrap();
        assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
    }

    /// A scheme the app set is its own: the terminal's reports and a late
    /// OSC 11 answer do not override it.
    #[test]
    fn an_app_set_scheme_is_not_overridden_by_the_terminal() {
        let (app, _) = app("");
        let mut app = app.with_color_scheme(ColorScheme::Dark);
        send(&mut app, b"\x1b[?997;2n\x1b]11;rgb:ffff/ffff/ffff\x07");
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
        assert_eq!(app.detected_background(), None);
    }

    /// An OSC 11 reply that arrives after the startup query stopped
    /// waiting is still the terminal's answer: kept, and its scheme
    /// preferred — not typed into the focused element.
    #[test]
    fn a_late_background_reply_is_applied() {
        let (mut app, _) = app("");
        send(&mut app, b"\x1b]11;rgb:fafa/fafa/f0f0\x1b\\");
        assert_eq!(app.detected_background(), Some(Color::Rgb(250, 250, 240)));
        assert_eq!(app.color_scheme(), ColorScheme::Light);
    }

    /// Dark until set — what a terminal that does not answer gets.
    #[test]
    fn the_default_scheme_is_dark() {
        let (app, _) = app("");
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
    }
}
