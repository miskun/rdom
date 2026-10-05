//! The box of a `::before` / `::after` laid out as a flex item (CSS
//! Flexbox §4, `GeneratedBox`): its background and border at its border
//! box, in its computed style with a running transition's paint values
//! overlaid, under its content (which `inline_paint` paints as the
//! item's anonymous box lines).

use rdom_core::{Dom, NodeId};

use super::background::paint_background;
use super::border::paint_border_sides;
use super::layout_rect_to_grid;
use crate::ext::{GeneratedBox, PseudoSlot, StyleSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::render::buffer::BorderContribution;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

/// Paint the box of every generated flex item of `container`.
pub(super) fn paint_generated_boxes(
    dom: &Dom<TuiExt>,
    container: NodeId,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(ext) = dom.node(container).ext() else {
        return;
    };
    for anon in &ext.anonymous_blocks {
        if let Some(g) = anon.generated {
            paint_box(dom, g, anon.rect, buf, clip);
        }
    }
}

fn paint_box(
    dom: &Dom<TuiExt>,
    g: GeneratedBox,
    content: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(ext) = dom.node(g.host).ext() else {
        return;
    };
    let (style, overrides, slot) = match g.slot {
        PseudoSlot::Before => (
            &ext.computed_before,
            ext.presentation_before.as_deref(),
            StyleSlot::Before,
        ),
        PseudoSlot::After => (
            &ext.computed_after,
            ext.presentation_after.as_deref(),
            StyleSlot::After,
        ),
    };
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
