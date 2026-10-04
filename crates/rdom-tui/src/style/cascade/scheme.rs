//! The document's preferred color scheme (CSS Color Adjust 1 §2.1):
//! the scheme an element with `color-scheme: normal` uses and the one
//! `light-dark()` picks by unless the element says otherwise. A
//! terminal app takes it from the terminal's background — the `App`
//! asks the terminal at startup, or is told (`App::with_color_scheme`)
//! — and falls back to dark.
//!
//! Stored as document data, like the viewport, so every cascade form
//! reads the same scheme.

use rdom_core::Dom;
use rdom_style::color::ColorScheme;

use crate::ext::TuiExt;

/// The document-data slot holding the scheme.
#[derive(Debug, Clone, Copy)]
struct DocumentColorScheme(ColorScheme);

/// The color scheme `dom` prefers (dark until set).
pub(crate) fn document_color_scheme(dom: &Dom<TuiExt>) -> ColorScheme {
    dom.document_data::<DocumentColorScheme>()
        .map_or_else(ColorScheme::default, |s| s.0)
}

/// Set the color scheme `dom` prefers.
pub(crate) fn set_document_color_scheme(dom: &mut Dom<TuiExt>, scheme: ColorScheme) {
    match dom.document_data_mut::<DocumentColorScheme>() {
        Some(slot) => slot.0 = scheme,
        None => {
            dom.set_document_data(DocumentColorScheme(scheme));
        }
    }
}
