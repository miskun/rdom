//! The resizer of a resizable box (CSS UI 4 §4.2, C12G-RESIZE-PICKER):
//! one cell — the bottom-right cell inside the box's border, over its
//! content or the end of a scrollbar, where browsers draw the resizer —
//! that paint marks with a `◢` grip and a press on which starts the
//! corner drag (`runtime::resize`). One answer for both, so the hot spot
//! is always the cell that shows it.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::Overflow;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

/// The grip glyph.
pub(crate) const GRIP: &str = "◢";

/// The cell of `id`'s resizer: the bottom-right cell of its padding box,
/// when its `resize` allows an axis and it is a scroll container (§4.2
/// applies to scroll containers only — not `overflow: visible`, nor
/// `clip`, which makes none); `None` otherwise or while it has no room.
pub(crate) fn cell(dom: &Dom<TuiExt>, id: NodeId) -> Option<(i32, i32)> {
    let ext = dom.node(id).ext()?;
    let style = ext.computed.as_deref()?;
    if style.ui.resize.axes() == (false, false) || !is_scroll_container(style) {
        return None;
    }
    let pb = crate::render::layout_pass::geometry::compute_padding_box(ext.layout, style.border);
    if pb.width == 0 || pb.height == 0 {
        return None;
    }
    Some((
        pb.x + i32::from(pb.width) - 1,
        pb.y + i32::from(pb.height) - 1,
    ))
}

/// Whether `style` makes a scroll container (CSS Overflow 3 §3.1).
fn is_scroll_container(style: &ComputedStyle) -> bool {
    [style.overflow_x, style.overflow_y]
        .iter()
        .any(|o| matches!(o, Overflow::Hidden | Overflow::Scroll | Overflow::Auto))
}

/// Draw `id`'s grip in its resizer cell, inside `clip`, in its `color`
/// over what is there (after its content and scrollbars).
pub(crate) fn paint(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some((x, y)) = cell(dom, id) else {
        return;
    };
    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
        return;
    };
    if !buf.area.intersection(clip).contains(x, y) {
        return;
    }
    if let Some(c) = buf.cell_mut(x, y) {
        c.set_symbol(GRIP);
        c.set_fg(computed.fg);
    }
}
