//! Drag-autoscroll (DRAG-AUTOSCROLL) — during an autoscrolling drag
//! (a captured drag that opted in, or a text-selection drag), holding
//! the pointer in a band at a scroll container's
//! edge (or past it) scrolls the container a few cells per tick.
//!
//! Owns the edge-zone / step constants, the one-shot resolution of
//! which container a drag autoscrolls, the per-tick step computation,
//! and the step application. The runtime (`app`) drives the ticks.

use rdom_core::NodeId;

use super::ScrollAxis;
use super::geometry::nearest_scroll_container;
use super::scroll::set_scroll;
use crate::TuiDom;
use crate::node::TuiNodeExt;

/// Rows near a scroll container's edge that arm/drive autoscroll — a *band*,
/// not a single row, so the user doesn't have to hug the exact edge. Includes
/// everything past the edge too.
const AUTOSCROLL_EDGE_ZONE: i32 = 2;
/// Max cells scrolled per tick. Speed ramps with how far the pointer is into /
/// past the edge zone, capped here so a big overshoot doesn't teleport.
const AUTOSCROLL_MAX_STEP: i32 = 3;

/// Hit-test `(x, y)` and walk up to the nearest vertical scroll container, or
/// `None` if the point hits nothing or no ancestor scrolls vertically.
fn scroll_container_from_hit(dom: &TuiDom, x: u16, y: u16) -> Option<NodeId> {
    use crate::runtime::hit_test::HitTestExt;
    let hit = dom.hit_test(x, y)?;
    nearest_scroll_container(dom, hit)
}

/// Resolve the vertical scroll container a drag should autoscroll
/// (DRAG-AUTOSCROLL). Called **once** when the autoscroll session arms; the
/// runtime then keeps the result **sticky** for the rest of the drag, so the
/// drag's `source` (the captured node, or a text-selection drag's anchor
/// container) scrolling out of view — or the pointer overshooting past the
/// container onto a sibling — never re-targets or disarms it.
///
/// Resolution prefers the **raw pointer** (held at an edge the pointer is still
/// inside the container, so a hit-test finds the nearest vertical scroll
/// container directly, independent of `source`). It falls back to
/// hit-testing the pointer **clamped into `source`'s box** — for a
/// descendant scroller whose owner is the source (the virtual table's
/// `<tbody>` inside the captured `<table>`).
pub(crate) fn resolve_autoscroll_container(
    dom: &TuiDom,
    source: NodeId,
    pointer: (u16, u16),
) -> Option<NodeId> {
    scroll_container_from_hit(dom, pointer.0, pointer.1).or_else(|| {
        let cap = dom.node(source).tui_ext()?.layout;
        if cap.width == 0 || cap.height == 0 {
            return None;
        }
        let cx = (pointer.0 as i32).clamp(cap.x, cap.x + cap.width as i32 - 1) as u16;
        let cy = (pointer.1 as i32).clamp(cap.y, cap.y + cap.height as i32 - 1) as u16;
        scroll_container_from_hit(dom, cx, cy)
    })
}

/// The signed scroll step for a *known* `container` given the pointer, or `None`
/// when the pointer isn't in an edge zone or the container can't scroll that way
/// (so the tick idles without disarming). The zone is [`AUTOSCROLL_EDGE_ZONE`]
/// cells deep at each edge plus everything beyond it; the step ramps with the
/// pointer's distance into/past the zone, capped at [`AUTOSCROLL_MAX_STEP`].
/// The vertical band wins when the pointer is in both.
pub(crate) fn autoscroll_step_for(
    dom: &TuiDom,
    container: NodeId,
    pointer: (u16, u16),
) -> Option<(ScrollAxis, i32)> {
    let ext = dom.node(container).tui_ext()?;
    let border = dom
        .node(container)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    let zone = AUTOSCROLL_EDGE_ZONE.max(1);
    // Along one axis: the step into / past the far edge, or out of the
    // near edge, when there is room to scroll that way.
    let band = |pos: i32, start: i32, len: u16, offset: usize, content: usize| -> Option<i32> {
        let last = start + len as i32 - 1;
        let into_far = pos - (last - (zone - 1));
        if into_far >= 0 && offset + usize::from(len) < content {
            return Some((into_far + 1).clamp(1, AUTOSCROLL_MAX_STEP));
        }
        let into_near = (start + (zone - 1)) - pos;
        if into_near >= 0 && offset > 0 {
            return Some(-((into_near + 1).clamp(1, AUTOSCROLL_MAX_STEP)));
        }
        None
    };
    if let Some(step) = band(
        pointer.1 as i32,
        pb.y,
        pb.height,
        ext.scroll_y,
        ext.scroll_content_height,
    ) {
        return Some((ScrollAxis::Vertical, step));
    }
    band(
        pointer.0 as i32,
        pb.x,
        pb.width,
        ext.scroll_x,
        ext.scroll_content_width,
    )
    .map(|step| (ScrollAxis::Horizontal, step))
}

/// Scroll `container` by `step` cells on `axis` (clamped); returns `true` if the
/// offset actually changed (so the caller knows it hasn't hit the limit). Reuses
/// [`set_scroll`], so the clamp + `scroll`-event dispatch are shared.
pub(crate) fn autoscroll_step(
    dom: &mut TuiDom,
    container: NodeId,
    axis: ScrollAxis,
    step: i32,
) -> bool {
    let before = match dom.node(container).tui_ext() {
        Some(e) => match axis {
            ScrollAxis::Vertical => e.scroll_y,
            ScrollAxis::Horizontal => e.scroll_x,
        },
        None => return false,
    } as i32;
    let after = set_scroll(dom, container, axis, before + step) as i32;
    after != before
}
