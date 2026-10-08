//! Document-wide facts the cascade records for layout and paint, so they
//! can skip a walk no element in the document needs (document data,
//! [`Dom::document_data`]).
//!
//! A flag about the elements is conservative across partial cascades, as
//! the bottom-up `TuiExt::tree_has_*` flags are: a cascade that adds what
//! it names sets it, and nothing clears it — a stale `true` costs a walk, a
//! stale `false` would lose output. A flag about the sheets is set by every
//! cascade run from the sheets it cascades with.

use rdom_core::Dom;

use crate::ext::TuiExt;

/// The document's flags.
#[derive(Debug, Default, Clone, Copy)]
struct DocumentFlags {
    /// An element has computed as a list item (`display: list-item`, CSS
    /// Display 3 §2.6): a marker may ride a descendant's first line
    /// (CSS Lists 3 §3.5).
    list_items: bool,
    /// The sheets of the last cascade style `::first-line` or
    /// `::first-letter` (CSS Pseudo-Elements 4 §2.2, §2.3).
    first_rules: bool,
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

/// Record whether the sheets being cascaded style `::first-line` or
/// `::first-letter`.
pub(crate) fn set_first_rules(dom: &mut Dom<TuiExt>, styled: bool) {
    if flags(dom).first_rules != styled {
        let mut next = flags(dom);
        next.first_rules = styled;
        dom.set_document_data(next);
    }
}

/// Whether the sheets of the last cascade style `::first-line` or
/// `::first-letter`: a first formatted line is looked for only then.
pub(crate) fn has_first_rules(dom: &Dom<TuiExt>) -> bool {
    flags(dom).first_rules
}
