//! Document-wide facts the cascade records for layout and paint, so they
//! can skip a walk no element in the document needs (document data,
//! [`Dom::document_data`]).
//!
//! A flag about the elements is conservative across partial cascades, as
//! the bottom-up `TuiExt::tree_has_*` flags are: a cascade that adds what
//! it names sets it, and nothing clears it — a stale `true` costs a walk, a
//! stale `false` would lose output — except `calc_sizes`, which the walk
//! it gates clears when it finds no such box. A flag about the sheets is set by every
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
    /// A cascade evaluated a `:has()` for some element (Selectors 4
    /// §4.5): the dirty tracker looks for anchors above a change only
    /// then.
    has_anchors: bool,
    /// A cascade evaluated a `:has()` for the root fragment — the root
    /// element, which has no `TuiExt` to hold `has_anchor`
    /// (C14G-ROOT-ELEMENT).
    root_has_anchor: bool,
    /// The sheets of the last cascade style `::first-line` or
    /// `::first-letter` (CSS Pseudo-Elements 4 §2.2, §2.3).
    first_rules: bool,
    /// An element's `width` or `height` has computed as a `calc-size()`
    /// (CSS Values 5 §10), authored or a running transition's value: layout
    /// sizes such boxes in a second pass (`layout_pass::calc_size`).
    calc_sizes: bool,
    /// An element has computed a `mix-blend-mode` other than `normal`
    /// (Compositing 1 §3.2): paint looks for the isolated groups its
    /// blending needs only then.
    blends: bool,
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

/// Whether `style`'s `width` or `height` is a `calc-size()`.
pub(crate) fn is_calc_sized(style: &crate::style::ComputedStyle) -> bool {
    use crate::layout::Size;
    matches!(style.width, Size::CalcSize(_)) || matches!(style.height, Size::CalcSize(_))
}

/// Record that an element's `width` or `height` is a `calc-size()`.
pub(crate) fn note_calc_size(dom: &mut Dom<TuiExt>) {
    if !flags(dom).calc_sizes {
        let mut next = flags(dom);
        next.calc_sizes = true;
        dom.set_document_data(next);
    }
}

/// Record that no element's `width` or `height` is a `calc-size()`:
/// layout's collecting walk found none (`layout_pass::calc_size`), so
/// later layouts skip it until a cascade or a composite notes one again.
pub(crate) fn clear_calc_sizes(dom: &mut Dom<TuiExt>) {
    if flags(dom).calc_sizes {
        let mut next = flags(dom);
        next.calc_sizes = false;
        dom.set_document_data(next);
    }
}

/// Whether an element's `width` or `height` may be a `calc-size()`
/// ([`note_calc_size`] since the document was first cascaded).
pub(crate) fn has_calc_sizes(dom: &Dom<TuiExt>) -> bool {
    flags(dom).calc_sizes
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

/// Record that an element of the document is a `:has()` anchor.
pub(crate) fn note_has_anchor(dom: &mut Dom<TuiExt>) {
    if !flags(dom).has_anchors {
        let mut next = flags(dom);
        next.has_anchors = true;
        dom.set_document_data(next);
    }
}

/// Record that the root fragment is a `:has()` anchor.
pub(crate) fn note_root_has_anchor(dom: &mut Dom<TuiExt>) {
    if !flags(dom).root_has_anchor {
        let mut next = flags(dom);
        next.root_has_anchor = true;
        dom.set_document_data(next);
    }
}

/// Whether the root fragment may be a `:has()` anchor
/// ([`note_root_has_anchor`] since the document was first cascaded).
pub(crate) fn root_has_anchor(dom: &Dom<TuiExt>) -> bool {
    flags(dom).root_has_anchor
}

/// Whether an element of the document may be a `:has()` anchor
/// ([`note_has_anchor`] since the document was first cascaded).
pub(crate) fn has_has_anchors(dom: &Dom<TuiExt>) -> bool {
    flags(dom).has_anchors
}

/// Record that an element of the document blends.
pub(crate) fn note_blend(dom: &mut Dom<TuiExt>) {
    if !flags(dom).blends {
        let mut next = flags(dom);
        next.blends = true;
        dom.set_document_data(next);
    }
}

/// Whether an element of the document may blend ([`note_blend`] since
/// the document was first cascaded).
pub(crate) fn has_blends(dom: &Dom<TuiExt>) -> bool {
    flags(dom).blends
}
