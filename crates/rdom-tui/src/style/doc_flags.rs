//! Document-wide facts the cascade records for layout and paint, so they
//! can skip a walk no element in the document needs (document data,
//! [`Dom::document_data`]).
//!
//! Each flag is conservative across partial cascades, as the bottom-up
//! `TuiExt::tree_has_*` flags are: a cascade that adds what a flag names
//! sets it, and nothing clears it — a stale `true` costs a walk, a stale
//! `false` would lose output.

use rdom_core::Dom;

use crate::ext::TuiExt;

/// The document's flags.
#[derive(Debug, Default, Clone, Copy)]
struct DocumentFlags {
    /// An element has computed as a list item (`display: list-item`, CSS
    /// Display 3 §2.6): a marker may ride a descendant's first line
    /// (CSS Lists 3 §3.5).
    list_items: bool,
}

fn flags(dom: &Dom<TuiExt>) -> DocumentFlags {
    dom.document_data::<DocumentFlags>()
        .copied()
        .unwrap_or_default()
}

/// Record that an element of the document is a list item.
pub(crate) fn note_list_item(dom: &mut Dom<TuiExt>) {
    if !flags(dom).list_items {
        let mut next = flags(dom);
        next.list_items = true;
        dom.set_document_data(next);
    }
}

/// Whether an element of the document may be a list item ([`note_list_item`]
/// since the document was first cascaded).
pub(crate) fn has_list_items(dom: &Dom<TuiExt>) -> bool {
    flags(dom).list_items
}
