//! The containment an element applies (CSS Containment 2 §3, Containment
//! 3, CSS Will Change 1 §3): from its `contain` and the containment its
//! `container-type` implies — and what `will-change` makes of it. The
//! layout, paint and cascade effects all ask here.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::Display;
use crate::style::ComputedStyle;

/// Size containment on the inline axis (horizontal-tb: the width): `contain:
/// size | inline-size`, or a `size` / `inline-size` query container (CSS
/// Conditional 5 §6.1).
pub(crate) fn size_inline(c: &ComputedStyle) -> bool {
    c.contain.size || c.contain.inline_size || c.container_type.queries_inline()
}

/// Size containment on the block axis: `contain: size`, or a `size`
/// query container.
pub(crate) fn size_block(c: &ComputedStyle) -> bool {
    c.contain.size || c.container_type.queries_block()
}

/// Size containment on the inline axis of the element `id` styled `c`:
/// [`size_inline`], or its contents skipped (`content-visibility`, §4).
pub(crate) fn size_inline_of(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> bool {
    size_inline(c) || super::content_visibility::skips(dom, id, c)
}

/// [`size_inline_of`] for the block axis.
pub(crate) fn size_block_of(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> bool {
    size_block(c) || super::content_visibility::skips(dom, id, c)
}

/// `content-visibility: auto | hidden` applies layout, style and paint
/// containment (§4).
fn content_visibility(c: &ComputedStyle) -> bool {
    c.content_visibility != crate::layout::ContentVisibility::Visible
}

/// Layout containment (§3.2).
pub(crate) fn layout(c: &ComputedStyle) -> bool {
    c.contain.layout || content_visibility(c)
}

/// Paint containment (§3.4).
pub(crate) fn paint(c: &ComputedStyle) -> bool {
    c.contain.paint || content_visibility(c)
}

/// Style containment (§3.3).
pub(crate) fn style(c: &ComputedStyle) -> bool {
    c.contain.style || content_visibility(c)
}

/// The properties whose non-initial value makes a stacking context (CSS
/// Will Change 1 §3: naming one in `will-change` makes one).
const STACKING: &[&str] = &[
    "opacity",
    "transform",
    "translate",
    "rotate",
    "scale",
    "perspective",
    "filter",
    "backdrop-filter",
    "clip-path",
    "mask",
    "mask-image",
    "mix-blend-mode",
    "isolation",
    "contain",
    "view-transition-name",
    "offset-path",
];

/// The properties whose non-initial value makes a containing block for
/// absolutely and fixed positioned descendants (§3: naming one makes
/// one).
const CONTAINING: &[&str] = &[
    "transform",
    "translate",
    "rotate",
    "scale",
    "perspective",
    "filter",
    "backdrop-filter",
    "contain",
    "offset-path",
];

/// Whether the element `c` establishes a stacking context through
/// containment: layout or paint containment (§3.2, §3.4), or a
/// `will-change` naming such a property.
pub(crate) fn makes_stacking_context(c: &ComputedStyle) -> bool {
    c.display != Display::Contents
        && (layout(c) || paint(c) || STACKING.iter().any(|p| c.will_change.names(p)))
}

/// Whether the element `c` is the containing block of its absolutely
/// positioned descendants — of its `fixed` ones too when `fixed` — beyond
/// what its `position` says: layout or paint containment (§3.2, §3.4),
/// or a `will-change` naming a property that would make it one (`position`
/// for the absolute ones only: a non-static `position` contains those).
pub(crate) fn contains_positioned(c: &ComputedStyle, fixed: bool) -> bool {
    c.display != Display::Contents
        && (layout(c)
            || paint(c)
            || CONTAINING.iter().any(|p| c.will_change.names(p))
            || (!fixed && c.will_change.names("position")))
}
