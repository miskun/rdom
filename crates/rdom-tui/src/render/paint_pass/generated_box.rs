//! The box of a `::before` / `::after` laid out as a box of its own — a
//! flex or grid item (CSS Flexbox §4, `GeneratedBox`), a block-level box,
//! an atomic inline or a float (CSS Pseudo 4 §2): its background and
//! border at its border box, in its computed style with a running
//! transition's paint values overlaid, under its content (which
//! `inline_paint` paints as the box's lines).

use rdom_core::{Dom, NodeId};

use super::background::paint_background;
use super::border::paint_border_sides;
use super::layout_rect_to_grid;
use crate::ext::{GeneratedBox, StyleSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::render::buffer::BorderContribution;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

/// Paint the box of every generated flex or grid item of `container`,
/// atomically with its content (CSS Flexbox §5.4). A block container's
/// block-level `::before` / `::after` paint their boxes in their unit's
/// background phase instead ([`paint_generated_block`]).
pub(super) fn paint_generated_boxes(
    dom: &Dom<TuiExt>,
    container: NodeId,
    buf: &mut Buffer,
    clip: Rect,
) {
    if !crate::render::box_tree::is_flex_or_grid_container(dom, container) {
        return;
    }
    let Some(ext) = dom.node(container).ext() else {
        return;
    };
    for anon in &ext.anonymous_blocks {
        if let Some(g) = anon.generated {
            paint_box(dom, g, anon.rect, buf, clip);
        }
    }
}

/// Paint the box of the block-level `::before` / `::after` that is
/// `host`'s `k`-th anonymous box, in its unit's background phase (CSS 2.1
/// Appendix E step 4).
pub(super) fn paint_generated_block(
    dom: &Dom<TuiExt>,
    host: NodeId,
    k: usize,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(anon) = dom.node(host).ext().and_then(|e| e.anonymous_blocks.get(k)) else {
        return;
    };
    if let Some(g) = anon.generated {
        paint_box(dom, g, anon.rect, buf, clip);
    }
}

pub(super) fn paint_box(
    dom: &Dom<TuiExt>,
    g: GeneratedBox,
    content: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(ext) = dom.node(g.host).ext() else {
        return;
    };
    let slot = StyleSlot::from(g.slot);
    let (style, overrides) = (ext.computed_pseudo(g.slot), ext.presentation_for(slot));
    let Some(style) = style else {
        return;
    };
    if !crate::render::visibility::shows(dom, g.host, slot) {
        return;
    }
    let mut style = ComputedStyle::clone(style);
    if let Some(o) = overrides {
        if let Some(bg) = o.bg {
            style.bg = bg;
        }
        if let Some(border_color) = o.border_color {
            style.border_color = border_color;
        }
    }
    let outer = g.border_box;
    let Some(outer_grid) = layout_rect_to_grid(outer, clip) else {
        return;
    };
    paint_background(buf, &style, outer, content, clip);
    if !style.border.is_empty() {
        // One level below its host, as a child box.
        let mut depth: u16 = 1;
        let mut cur = dom.node(g.host).parent_node();
        while let Some(node) = cur {
            depth = depth.saturating_add(1);
            cur = node.parent_node();
        }
        let priority = BorderContribution::pack_priority(depth, g.host.as_u32());
        paint_border_sides(buf, &style, outer, outer_grid, clip, priority);
    }
}
