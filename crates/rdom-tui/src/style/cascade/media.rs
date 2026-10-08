//! The document's media environment (Media Queries 4 / 5): what `@media`
//! and `matchMedia` evaluate against — the viewport and the preferred
//! color scheme (each its own document data, `viewport.rs` and
//! `scheme.rs`) and the preferences an app reports
//! ([`MediaPreferences`], stored here, a terminal's defaults until set).

use rdom_core::Dom;
use rdom_style::conditional::{MediaEnvironment, MediaPreferences};

use crate::ext::TuiExt;

/// The document-data slot holding the preferences.
#[derive(Debug, Clone, Copy)]
struct DocumentMediaPreferences(MediaPreferences);

/// The media preferences `dom` reports (the defaults until set).
pub(crate) fn document_media_preferences(dom: &Dom<TuiExt>) -> MediaPreferences {
    dom.document_data::<DocumentMediaPreferences>()
        .map_or_else(MediaPreferences::default, |p| p.0)
}

/// Set the media preferences `dom` reports.
pub(crate) fn set_document_media_preferences(dom: &mut Dom<TuiExt>, prefs: MediaPreferences) {
    match dom.document_data_mut::<DocumentMediaPreferences>() {
        Some(slot) => slot.0 = prefs,
        None => {
            dom.set_document_data(DocumentMediaPreferences(prefs));
        }
    }
}

/// The environment `dom`'s media queries evaluate in.
pub(crate) fn document_media(dom: &Dom<TuiExt>) -> MediaEnvironment {
    MediaEnvironment::new(
        super::document_viewport(dom),
        super::document_color_scheme(dom),
        document_media_preferences(dom),
    )
}
