//! Atomic inline `::before` / `::after` (CSS Pseudo 4 §2: a
//! pseudo-element's `display` makes its box as an element's does; CSS
//! Display 3 §2.4): an `inline-block`, `inline flow-root`, `inline-flex`
//! or `inline-grid` one is one box in its host's line, as an element atom
//! is (CSS 2.1 §10.8). It is the box tree's generated item
//! (`items::AnonymousItem::pseudo`): measured here for the packer, which
//! places it in its line as a generated fragment holding its box
//! (`inline::GeneratedFragment::is_atom`), and its content laid out
//! inside it once the line is packed — its text, or its one anonymous item
//! by flex or grid layout.

use rdom_core::{Dom, NodeId};

use super::items::AnonymousItem;
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::render::inline::InlineLayout;
use crate::render::inline::vertical::AtomRows;

/// The width and rows of `host`'s atomic inline `slot` pseudo-element
/// in a line whose content box — its containing block — is `cb_width`
/// wide, as an element atom's are measured (`inline::feed`): its
/// max-content border box (DIVERGENCES §2) and its height at that width,
/// its baseline its last content row. A `measuring` packer asks for its
/// width only. `None` when it has no computed style.
pub(crate) fn measure(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    cb_width: u16,
    measuring: bool,
) -> Option<(u16, AtomRows)> {
    let item = AnonymousItem::pseudo(dom, host, slot)?;
    let width = item.box_size(dom, Direction::Row, 0, cb_width, true);
    if measuring {
        return Some((width, AtomRows::UNMEASURED));
    }
    let height = item.box_size(dom, Direction::Column, width, cb_width, true);
    let last = item
        .content_rows(dom, width, cb_width)
        .map(|(_, last)| last);
    Some((width, AtomRows::of(item.style(), height, cb_width, last)))
}

/// Lay out the content of each atomic pseudo-element on `layout`'s lines
/// inside its box, its percentages against the lines' content width (its
/// containing block's): kept on its fragment, from its border box's
/// top-left corner, for paint.
pub(super) fn lay_out(dom: &mut Dom<TuiExt>, layout: &mut InlineLayout) {
    let cb_width = layout.content_width;
    for line in &mut layout.lines {
        for g in &mut line.generated {
            let Some(atom) = g.atom.as_mut() else {
                continue;
            };
            let Some(item) = AnonymousItem::pseudo(dom, g.host, g.slot) else {
                continue;
            };
            let border_box = LayoutRect::new(0, 0, g.width, atom.height);
            let content = item.content_rect(border_box, cb_width);
            atom.content = Some(item.lay_out_content(dom, content));
        }
    }
}
