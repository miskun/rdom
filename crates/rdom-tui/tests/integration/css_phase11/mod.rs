//! CSS-COMPLETE Phase 11 — selectors: each sheet parsed strictly and
//! cascaded through rdom-css, the claim read from the computed style
//! (or the paint, where the cells are the claim). One submodule per
//! item; each test cites the Selectors 4 section that fixes the result.

use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::{App, CascadeExt, Color, NodeId, TuiDom, TuiNodeExt};

#[allow(unused_imports)]
pub(crate) use super::css_phase5::el;

mod attr_flags;
mod form_states;
mod has;
mod link_lang;
mod modal;
mod nth;
mod popover;

/// Parse `css` strictly (no warnings) and cascade it over `dom`.
pub(crate) fn cascade(dom: &mut TuiDom, css: &str) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
}

/// `id`'s computed foreground color.
pub(crate) fn fg(dom: &TuiDom, id: NodeId) -> Color {
    dom.node(id).computed().expect("cascaded").fg
}

/// The terminal's default foreground: what an unstyled element computes.
pub(crate) const UNSTYLED: Color = Color::Reset;

pub(crate) const RED: Color = Color::Rgb(255, 0, 0);
pub(crate) const BLUE: Color = Color::Rgb(0, 0, 255);

/// An App over `dom` with `css` (parsed strictly), one frame drawn in a
/// 20 × 10 terminal: the incremental path — the dirty tracker narrowed
/// by the App's sheets, `cascade_subtrees` per frame.
pub(crate) fn app(dom: TuiDom, css: &str) -> App<TestBackend> {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    let terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

/// `id`'s computed foreground in `app`.
pub(crate) fn app_fg(app: &App<TestBackend>, id: NodeId) -> Color {
    fg(app.dom(), id)
}
