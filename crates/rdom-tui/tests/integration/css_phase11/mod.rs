//! CSS-COMPLETE Phase 11 — selectors: each sheet parsed strictly and
//! cascaded through rdom-css, the claim read from the computed style
//! (or the paint, where the cells are the claim). One submodule per
//! item; each test cites the Selectors 4 section that fixes the result.

use rdom_tui::{CascadeExt, Color, NodeId, TuiDom, TuiNodeExt};

#[allow(unused_imports)]
pub(crate) use super::css_phase5::el;

mod attr_flags;

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
