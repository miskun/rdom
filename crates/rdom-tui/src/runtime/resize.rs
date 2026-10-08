//! Resizing a box by its corner (CSS UI 4 §4.2 `resize`): a press on the
//! bottom-right cell of a scroll container whose `resize` is not `none`
//! starts a drag that takes the pointer; each move sets the box's
//! `width` / `height` — on the axes `resize` allows — in its `style`
//! attribute, as browsers write the resized size into the element's
//! inline style; the release (or a lost one, as for the scrollbar thumb)
//! ends it. The box's border box follows the pointer, its content box
//! never below one cell. The press must be on the resizer cell
//! (`render::resizer`), which paint marks with a `◢` grip.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{BoxSizing, Size};
use crate::node::TuiNodeMutExt;
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
    /// The size each axis holds now — the one last written, at first
    /// the laid-out one: a move writes only an axis whose size it changes.
    written: (u16, u16),
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

/// The drag `id` would start from a press at `(x, y)`: on its resizer
/// cell (`render::resizer`, the cell that shows the grip).
fn at_corner(dom: &TuiDom, id: NodeId, x: u16, y: u16) -> Option<ResizeDrag> {
    if crate::render::resizer::cell(dom, id)? != (i32::from(x), i32::from(y)) {
        return None;
    }
    let ext = dom.node(id).ext()?;
    let style = ext.computed.as_deref()?;
    let r = ext.layout;
    let extra = if style.box_sizing == BoxSizing::BorderBox {
        (0, 0)
    } else {
        // What a `content-box` size leaves out, as laid out: padding
        // (resolved against the containing block) and border — the
        // border box less the content box, less the gutters, which the
        // content box gives up inside the size.
        let g = crate::runtime::scrollbar::state::gutters(ext);
        let c = ext.content_layout;
        (
            r.width.saturating_sub(c.width).saturating_sub(g.columns()),
            r.height.saturating_sub(c.height).saturating_sub(g.bottom),
        )
    };
    let content = (
        r.width.saturating_sub(extra.0),
        r.height.saturating_sub(extra.1),
    );
    Some(ResizeDrag {
        element: id,
        from: (x, y),
        size: (r.width, r.height),
        extra,
        axes: style.ui.resize.axes(),
        written: content,
    })
}

/// A move to `(x, y)` during a drag: write the size it gives. Whether it
/// changed the element's style.
pub(crate) fn extend(router: &mut Router, dom: &mut TuiDom, x: u16, y: u16) -> bool {
    let Some(drag) = router.resize_drag.as_mut() else {
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
    let width = size(drag.size.0, drag.from.0, x, drag.extra.0);
    let height = size(drag.size.1, drag.from.1, y, drag.extra.1);
    let mut changed = false;
    let mut node = dom.node_mut(drag.element);
    // Only an axis `resize` allows and the move changed: each write is a
    // mutation, a restyle and a layout.
    if drag.axes.0 && width != drag.written.0 {
        node.set_width(Size::Fixed(width));
        drag.written.0 = width;
        changed = true;
    }
    if drag.axes.1 && height != drag.written.1 {
        node.set_height(Size::Fixed(height));
        drag.written.1 = height;
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
