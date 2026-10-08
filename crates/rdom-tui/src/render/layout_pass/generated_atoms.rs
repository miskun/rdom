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
/// wide, as an element atom's are (`inline::feed::push_atom`): its
/// shrink-to-fit border box (CSS 2.1 §10.3.9, `float::size::FloatBox`) and
/// its baseline its last content row. An intrinsic width measurement
/// (`measuring`: `Some(max_content)`) asks for its contribution under that
/// constraint only. `None` when it has no computed style.
pub(crate) fn measure(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    cb_width: u16,
    measuring: Option<bool>,
) -> Option<(u16, AtomRows)> {
    let item = AnonymousItem::pseudo(dom, host, slot)?;
    if let Some(max_content) = measuring {
        let width = item.box_size(dom, Direction::Row, 0, 0, max_content);
        return Some((width, AtomRows::UNMEASURED));
    }
    let fb = crate::render::layout_pass::float::size::FloatBox::of(
        dom,
        crate::render::box_tree::BoxItem::Generated(host, slot),
        cb_width,
    );
    let last = item
        .content_rows(dom, fb.width, cb_width)
        .map(|(_, last)| last);
    Some((
        fb.width,
        AtomRows::of(item.style(), fb.height, cb_width, last),
    ))
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
            atom.content = Some(item.lay_out_content(dom, content, border_box));
        }
    }
}
