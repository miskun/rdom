//! CSS Object Model surface for `rdom-tui`.
//!
//! This module is the canonical home for code that bridges
//! [`rdom_css`] (the CSS parser, leaf with no `rdom-tui` deps) and
//! [`crate::TuiDom`] (the tree being styled). The rule: anything
//! that needs both crates lands here, not scattered through the
//! tree.
//!
//! ## What lives here
//!
//! - [`apply`] — parse-and-apply helpers run before `App::new`:
//!   [`extend_from_style_tags`] walks `<style>` blocks into a
//!   `Stylesheet`; [`seed_inline_styles`] writes `style="…"`
//!   attribute values into each element's `TuiExt::inline_style`.
//! - **M4b step 26 (coming)** — `StyleDeclaration` /
//!   `StyleDeclarationMut`: the IDL-style CSSOM wrapper around
//!   `TuiStyle` that drives `element.style.color = "red"` /
//!   `element.style.cssText` semantics, consuming
//!   [`rdom_style::property_dispatch`].
//!
//! ## Why a dedicated module
//!
//! Pre-rdom-style-extraction these helpers lived in `rdom-css`,
//! which forced `rdom-css → rdom-tui` (because the helpers need
//! `TuiDom`). The mid-stream restructure pushed the CSS data model
//! down into the `rdom-style` leaf, and then the helpers moved up
//! here so `rdom-css` becomes a pure parser (no `TuiDom`
//! knowledge). Final dep direction:
//!
//! ```text
//! rdom-style ← rdom-css
//!            ← rdom-tui (depends on both — owns cssom/ glue)
//! ```

pub mod apply;
pub mod declaration;
pub mod observer;

// Build-script-generated camelCase aliases on `StyleDeclaration`
// — `el.style().color()`, `el.style().background_color()`, etc.
// One method per name in `property_dispatch::property_names()`.
mod aliases;
pub(crate) mod reentry;

pub use apply::{extend_from_style_tags, seed_inline_styles};
pub use declaration::{SetPropertyError, StyleDeclaration, StyleDeclarationMut};
pub use observer::install as install_inline_style_observer;
pub use rdom_style::property_dispatch::DispatchError;

/// Install every default CSSOM observer that a typical
/// `rdom-tui` app needs. Today that's only the inline-style
/// observer ([`install_inline_style_observer`]) — the one that
/// refreshes [`crate::TuiExt::inline_style`] whenever the
/// `style="…"` attribute mutates after build. Apps that
/// construct a [`crate::TuiDom`] directly (without going through
/// [`crate::App::build`]) should call this once before they
/// expect inline-style mutations to flow through to the cascade,
/// otherwise the old `D-M1-4` symptom recurs: programmatic
/// `set_attribute("style", "…")` writes don't update the typed
/// `inline_style` field.
///
/// [`crate::App::build`] calls this internally; you only need to
/// call it directly when bypassing `App`.
pub fn install_default_observers(dom: &mut rdom_core::Dom<crate::TuiExt>) {
    let _ = install_inline_style_observer(dom);
}
