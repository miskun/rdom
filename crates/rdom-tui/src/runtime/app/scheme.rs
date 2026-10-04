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
    /// startup.
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

    /// At startup, unless the app set a scheme: ask the terminal for
    /// its background (OSC 11) and prefer the scheme it calls for.
    pub(super) fn detect_color_scheme(&mut self) {
        if self.color_scheme_explicit {
            return;
        }
        if let Some(scheme) = crate::runtime::color_scheme::query_terminal_scheme(
            crate::runtime::color_scheme::QUERY_TIMEOUT,
        ) {
            self.dom.set_color_scheme(scheme);
            self.redraw.note(Redraw::Cascade);
        }
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

    /// Dark until set — what a terminal that does not answer gets.
    #[test]
    fn the_default_scheme_is_dark() {
        let (app, _) = app("");
        assert_eq!(app.color_scheme(), ColorScheme::Dark);
    }
}
