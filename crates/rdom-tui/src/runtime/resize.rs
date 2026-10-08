//! Resizing a box by its corner (CSS UI 4 §4.2 `resize`): a press on the
//! bottom-right cell of a scroll container whose `resize` is not `none`
//! starts a drag that takes the pointer; each move sets the box's
//! `width` / `height` — on the axes `resize` allows — in its `style`
//! attribute, as browsers write the resized size into the element's
//! inline style; the release (or a lost one, as for the scrollbar thumb)
//! ends it. The box's border box follows the pointer, its content box
//! never below one cell; no grip glyph is drawn (DIVERGENCES §2).

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{BoxSizing, Overflow, Size};
use crate::node::{TuiNodeExt, TuiNodeMutExt};
use crate::runtime::router::Router;

/// A corner drag in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResizeDrag {
    element: NodeId,
    /// Where the press was.
    from: (u16, u16),
    /// The border box then.
    size: (u16, u16),
    /// What the border box adds to the content box on each axis, for a
    /// `content-box` element (zero for `border-box`).
    extra: (u16, u16),
    /// The axes `resize` allows: horizontal, vertical.
    axes: (bool, bool),
}

/// A press at `(x, y)` on the hit `path`: start a corner drag when it is
/// on the bottom-right cell of a resizable scroll container on the path
/// (the innermost such one) — taking the pointer. Whether it started.
pub(crate) fn begin(
    router: &mut Router,
    dom: &mut TuiDom,
    path: &[NodeId],
    x: u16,
    y: u16,
) -> bool {
    let Some(drag) = path.iter().rev().find_map(|&id| at_corner(dom, id, x, y)) else {
        return false;
    };
    // A hit element is connected, so it can take the pointer; one that
    // cannot starts no drag.
    if dom.set_pointer_capture(drag.element).is_err() {
        return false;
    }
    router.resize_drag = Some(drag);
    true
}

/// The drag `id` would start from a press at `(x, y)`.
fn at_corner(dom: &TuiDom, id: NodeId, x: u16, y: u16) -> Option<ResizeDrag> {
    let node = dom.node(id);
    let style = node.computed()?;
    let axes = style.ui.resize.axes();
    // §4.2: `resize` applies to elements with an `overflow` other than
    // `visible`.
    let clips = style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;
    if axes == (false, false) || !clips {
        return None;
    }
    let r = node.layout_rect()?;
    if r.width == 0 || r.height == 0 {
        return None;
    }
    let corner = (r.x + i32::from(r.width) - 1, r.y + i32::from(r.height) - 1);
    if corner != (i32::from(x), i32::from(y)) {
        return None;
    }
    let extra = if style.box_sizing == BoxSizing::BorderBox {
        (0, 0)
    } else {
        // Padding and border: what a `content-box` width leaves out.
        let pad = |v: &crate::layout::PaddingValue| v.resolve(r.width);
        let side = |s: crate::layout::BorderStyle| u16::from(s.is_visible());
        let (p, b) = (&style.padding, style.border);
        (
            pad(&p.left) + pad(&p.right) + side(b.left) + side(b.right),
            pad(&p.top) + pad(&p.bottom) + side(b.top) + side(b.bottom),
        )
    };
    Some(ResizeDrag {
        element: id,
        from: (x, y),
        size: (r.width, r.height),
        extra,
        axes,
    })
}

/// A move to `(x, y)` during a drag: write the size it gives. Whether it
/// changed the element's style.
pub(crate) fn extend(router: &Router, dom: &mut TuiDom, x: u16, y: u16) -> bool {
    let Some(drag) = router.resize_drag else {
        return false;
    };
    if !dom.contains(drag.element) {
        return false;
    }
    let size = |start: u16, from: u16, to: u16, extra: u16| -> u16 {
        let border = i32::from(start) + i32::from(to) - i32::from(from);
        // The content box never below one cell.
        let content = (border - i32::from(extra)).max(1);
        u16::try_from(content).unwrap_or(u16::MAX)
    };
    let mut changed = false;
    let mut node = dom.node_mut(drag.element);
    if drag.axes.0 {
        node.set_width(Size::Fixed(size(drag.size.0, drag.from.0, x, drag.extra.0)));
        changed = true;
    }
    if drag.axes.1 {
        node.set_height(Size::Fixed(size(drag.size.1, drag.from.1, y, drag.extra.1)));
        changed = true;
    }
    changed
}

/// End the drag (its release): the pointer capture is released with the
/// release's own.
pub(crate) fn end(router: &mut Router) {
    router.resize_drag = None;
}

/// End a drag whose release never arrived, dropping the capture it took
/// (every press does this first, as for the scrollbar thumb).
pub(crate) fn cancel(router: &mut Router, dom: &mut TuiDom) {
    if let Some(drag) = router.resize_drag.take()
        && dom.pointer_capture() == Some(drag.element)
    {
        dom.release_pointer_capture();
    }
}
