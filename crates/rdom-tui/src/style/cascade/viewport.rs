//! The document's viewport (CSS Values 4 §6.1.2: viewport-percentage
//! lengths are relative to the initial containing block, the size of
//! the viewport the document is presented in).
//!
//! Stored as document data (`rdom_core::Dom::set_document_data`), so
//! every cascade form reads the same size: `CascadeExt::set_viewport`
//! sets it, `LayoutExt::layout_dom(area)` records its area, the `App`
//! sets its terminal's size each frame. 0 × 0 until one of them runs.

use rdom_core::Dom;
use rdom_style::calc::Viewport;

use crate::ext::TuiExt;

/// The document-data slot holding the viewport.
#[derive(Debug, Clone, Copy)]
struct DocumentViewport(Viewport);

/// The viewport `dom`'s style resolves against.
pub(crate) fn document_viewport(dom: &Dom<TuiExt>) -> Viewport {
    dom.document_data::<DocumentViewport>()
        .map_or_else(Viewport::default, |v| v.0)
}

/// Set the viewport `dom`'s style resolves against.
pub(crate) fn set_document_viewport(dom: &mut Dom<TuiExt>, viewport: Viewport) {
    match dom.document_data_mut::<DocumentViewport>() {
        Some(slot) => slot.0 = viewport,
        None => {
            dom.set_document_data(DocumentViewport(viewport));
        }
    }
}
