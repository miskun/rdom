//! The containing block of an absolutely positioned box (CSS 2.1 §10.1,
//! CSS Position 3 §2.1) — one ancestor walk for elements and for
//! positioned `::before` / `::after`.
//!
//! - `fixed` → the viewport.
//! - `absolute` → the nearest ancestor box whose `position` is not
//!   `static` (`relative`, `absolute`, `fixed` and `sticky` alike): its
//!   padding box less its scrollbar gutters ([`padding_box`]); when it
//!   is a grid container, the grid area the box's placement properties
//!   name (CSS Grid 2 §9.1); when it is a scroll container, in its
//!   scrolled content, so the box scrolls with the content (CSS Overflow
//!   3 §2.2); the viewport when no ancestor qualifies.

use rdom_core::{Dom, NodeId};

use super::{computed_position, parent_id};
use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position};
use crate::style::ComputedStyle;

/// The containing block of an absolutely or fixed positioned box styled
/// `style`, whose ancestors are `from` and up: an element's box parent,
/// or a pseudo-element's host (CSS Pseudo-Elements 4 §4: `::before` /
/// `::after` are children of their originating element).
pub(in crate::render::layout_pass) fn absolute_containing_block(
    dom: &Dom<TuiExt>,
    from: Option<NodeId>,
    style: &ComputedStyle,
    viewport: LayoutRect,
) -> LayoutRect {
    if style.position == Position::Fixed {
        return viewport;
    }
    match containing_ancestor(dom, from) {
        Some(p) => of_ancestor(dom, p, style).unwrap_or(viewport),
        None => viewport,
    }
}

/// The ancestor that contains an absolutely positioned box whose
/// ancestors are `from` and up: the nearest one whose `position` is not
/// `static` (CSS Position 3 §2); `None` for the viewport. The one walk
/// placement and the scrollable overflow (`positioned_overflow`) share.
pub(in crate::render::layout_pass) fn containing_ancestor(
    dom: &Dom<TuiExt>,
    from: Option<NodeId>,
) -> Option<NodeId> {
    let mut cur = from;
    while let Some(p) = cur {
        if establishes_containing_block(computed_position(dom, p)) {
            return Some(p);
        }
        cur = parent_id(dom, p);
    }
    None
}

/// Whether a box with `position` contains its absolutely positioned
/// descendants (CSS Position 3 §2: every value but `static`).
fn establishes_containing_block(position: Position) -> bool {
    position != Position::Static
}

/// The containing block the positioned ancestor `p` gives a box styled
/// `style`: its padding box, a grid area within it, scrolled with `p`'s
/// content.
fn of_ancestor(dom: &Dom<TuiExt>, p: NodeId, style: &ComputedStyle) -> Option<LayoutRect> {
    let padding = padding_box(dom, p)?;
    let area =
        crate::render::layout_pass::grid::abspos_area(dom, style, p, padding).unwrap_or(padding);
    let (dx, dy) = scroll_offset(dom, p);
    Some(LayoutRect::new(
        area.x - dx,
        area.y - dy,
        area.width,
        area.height,
    ))
}

/// `p`'s scroll offset when it is a scroll container: its content —
/// and the boxes it contains — moved up and left by it.
fn scroll_offset(dom: &Dom<TuiExt>, p: NodeId) -> (i32, i32) {
    let Some(ext) = dom.node(p).ext() else {
        return (0, 0);
    };
    let scrolls = ext
        .computed
        .as_ref()
        .is_some_and(|c| c.is_scroll_container());
    if scrolls {
        (ext.scroll_x, ext.scroll_y)
    } else {
        (0, 0)
    }
}

/// `id`'s padding box — its border box less its border (CSS 2.1 §10.1:
/// the containing block a positioned box gives is "formed by the padding
/// edge of the ancestor") and less the scrollbar gutters it reserves (CSS
/// Overflow 3 §5.2: a gutter lies between the inner border edge and the
/// outer padding edge): its scrollport (`scrollport`), the gutters the
/// ones layout reserved.
fn padding_box(dom: &Dom<TuiExt>, id: NodeId) -> Option<LayoutRect> {
    let ext = dom.node(id).ext()?;
    let Some(c) = ext.computed.as_deref() else {
        return Some(ext.layout);
    };
    Some(crate::render::layout_pass::scrollport_of(ext, c))
}
