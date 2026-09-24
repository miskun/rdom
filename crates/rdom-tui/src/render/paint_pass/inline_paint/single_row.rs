//! The single-row painter: `::before` + a body string + `::after` on
//! an element's first content row, then the `<a href>` tag over the
//! painted span. Used for chrome substitution (gauge bar, closed
//! dropdown label, password mask) and for elements with no own text
//! at all — everything that is one row by construction.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::paint_pass::layout_rect_to_grid;
use crate::render::paint_pass::text::{paint_text_from, pseudo_style};
use crate::render::{Buffer, Rect, Style};

use super::{anchor_href_for, presentation_of};

/// Single-row paint: `::before` + `body_text` + `::after`. Used by
/// chrome-substitution elements (gauge, select, password mask) and
/// by elements with no own text at all.
pub(super) fn paint_single_row_chrome(
    dom: &Dom<TuiExt>,
    id: NodeId,
    body_text: &str,
    body_style: Style,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(inner_grid) = layout_rect_to_grid(inner, clip) else {
        return;
    };
    // Paint on the element's FIRST content row (`inner.y`), NOT the
    // clip-clamped top (`inner_grid.y`). For a height-1 chrome element
    // the two coincide, but a mixed-content block's `inner` spans its
    // block children, so clamping would drop the leading text run onto
    // whatever scrolled into the clip's top row (the "LaHello World"
    // bleed). If that row is scrolled out of the clip, skip it — it's
    // genuinely off-screen. Horizontal extent stays clipped via
    // `inner_grid`.
    if inner.y < clip.y as i32 || inner.y >= clip.bottom() as i32 {
        return;
    }
    let base_y = inner.y as u16;
    // Logical cursor: the run starts at `inner.x` even when that is left
    // of the clip; `paint_text_from` skips the clipped prefix.
    let mut cursor_x: i32 = inner.x;
    let budget_right = inner_grid.right();

    if let Some(before) = dom.node(id).computed_before()
        && before.position == crate::layout::Position::Static
        && let Some(ref text) = before.content
    {
        cursor_x = paint_text_from(
            buf,
            cursor_x,
            base_y,
            clip.x,
            budget_right,
            text,
            pseudo_style(
                before,
                presentation_of(dom, id, crate::ext::StyleSlot::Before),
            ),
        );
    }

    if !body_text.is_empty() {
        cursor_x = paint_text_from(
            buf,
            cursor_x,
            base_y,
            clip.x,
            budget_right,
            body_text,
            body_style,
        );
    }

    if let Some(after) = dom.node(id).computed_after()
        && after.position == crate::layout::Position::Static
        && let Some(ref text) = after.content
    {
        cursor_x = paint_text_from(
            buf,
            cursor_x,
            base_y,
            clip.x,
            budget_right,
            text,
            pseudo_style(
                after,
                presentation_of(dom, id, crate::ext::StyleSlot::After),
            ),
        );
    }

    if let Some(href) = anchor_href_for(dom, id) {
        // The hyperlink covers the painted span: from the clipped start to
        // the logical end, capped at the paint budget.
        let start = inner_grid.x;
        let end = cursor_x.min(i32::from(budget_right));
        let width = (end - i32::from(start)).max(0) as u16;
        if width > 0 {
            buf.set_link_range(start, base_y, width, Some(&href));
        }
    }
}
