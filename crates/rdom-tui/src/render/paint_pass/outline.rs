//! Outlines (CSS UI 4 §5): a ring of box-drawing cells around a box's
//! border edge, `outline-offset` cells out, that takes no room. Drawn
//! last in its stacking context (CSS 2.1 Appendix E step 10): the box
//! paints record their outline on the buffer ([`defer`]) and the
//! stacking-context walk draws the ones recorded since it began when it
//! ends ([`paint_since`]), each clipped as the box was — by its `overflow`
//! ancestors.
//!
//! The ring's glyphs are the border glyph set's (`border_join::glyphs`):
//! `double`, `dashed` / `dotted` runs, light or heavy single lines by
//! `outline-width` (a pixel width selects the weight, DESIGN "Pixel
//! lengths select, cells measure"), the 3-D styles as a single line; and
//! `auto` the UA focus-ring look — a light line with rounded corners in
//! the accent color. A ring cell replaces what is under it, a border's
//! junction included (its border state is cleared, as content does).

use rdom_style::color::SystemColor;
use rdom_style::layout::{BorderStyle, BorderWeight, OutlineColor, OutlineStyle};

use rdom_core::Dom;

use super::border_join::outline_glyph;
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

/// One box's outline, recorded at its box paint for its stacking
/// context's end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DeferredOutline {
    /// The ring's cells: the border box grown by the offset plus one.
    ring: LayoutRect,
    /// The clip the box painted into.
    clip: Rect,
    line: BorderStyle,
    weight: BorderWeight,
    rounded: bool,
    color: Color,
}

/// Record the outline of a box of `dom` with border box `outer`, styled
/// `style` and painted into `clip` — nothing for `outline-style: none` or
/// a zero width.
pub(super) fn defer(
    dom: &Dom<TuiExt>,
    buf: &mut Buffer,
    style: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
) {
    let ui = &style.ui;
    let Some(line) = ui.outline_style.line() else {
        return;
    };
    let Some(weight) = ui.outline_width.weight() else {
        return;
    };
    // §5.4: the offset moves the ring out from the border edge; the ring
    // is the cell beyond it.
    let out = ui.outline_offset.offset_cells().saturating_add(1);
    let grow = |extent: u16| i32::from(extent) + 2 * out;
    let (width, height) = (grow(outer.width), grow(outer.height));
    if width < 1 || height < 1 {
        return;
    }
    let ring = LayoutRect {
        x: outer.x.saturating_sub(out),
        y: outer.y.saturating_sub(out),
        width: width.min(i32::from(u16::MAX)) as u16,
        height: height.min(i32::from(u16::MAX)) as u16,
    };
    let auto = ui.outline_style == OutlineStyle::Auto;
    // §5.3: `auto` is the focus-ring color under `outline-style: auto`,
    // else `currentcolor`.
    let color = match &ui.outline_color {
        OutlineColor::Auto if auto => SystemColor::AccentColor.color(),
        OutlineColor::Auto => style.fg,
        OutlineColor::Color(c) => {
            // Under the element's used color scheme (`light-dark()`).
            let scheme = style
                .color_scheme
                .used(crate::style::CascadeExt::color_scheme(dom));
            let cx = crate::ColorContext::new(style.fg).with_scheme(scheme);
            c.resolve(&style.vars, &cx).unwrap_or(style.fg)
        }
    };
    if color.alpha() == 0 {
        return;
    }
    buf.outlines.push(DeferredOutline {
        ring,
        clip,
        line,
        weight,
        rounded: auto,
        color,
    });
}

/// How many outlines `buf` holds: a stacking context's walk takes it on
/// entry and [`paint_since`] it on exit.
pub(super) fn mark(buf: &Buffer) -> usize {
    buf.outlines.len()
}

/// Draw — and drop — the outlines recorded since `mark`, in the order
/// their boxes painted.
pub(super) fn paint_since(buf: &mut Buffer, mark: usize) {
    if buf.outlines.len() <= mark {
        return;
    }
    let outlines = buf.outlines.split_off(mark);
    for o in outlines {
        paint_ring(buf, &o);
    }
}

/// Draw one ring: each cell joins the ring cells beside it, so a ring
/// one cell wide or tall (a negative offset on a small box) is a line.
fn paint_ring(buf: &mut Buffer, o: &DeferredOutline) {
    let r = o.ring;
    let (left, top) = (r.x, r.y);
    let (right, bottom) = (r.x + i32::from(r.width) - 1, r.y + i32::from(r.height) - 1);
    let area = buf.area.intersection(o.clip);
    let put = |buf: &mut Buffer, x: i32, y: i32| {
        let (Ok(xu), Ok(yu)) = (u16::try_from(x), u16::try_from(y)) else {
            return;
        };
        if !area.contains(xu, yu) {
            return;
        }
        let side_col = x == left || x == right;
        let side_row = y == top || y == bottom;
        // N, E, S, W.
        let joins = [
            side_col && y > top,
            side_row && x < right,
            side_col && y < bottom,
            side_row && x > left,
        ];
        let Some(glyph) = outline_glyph(joins, o.line, o.weight, o.rounded) else {
            return;
        };
        buf.clear_border_at(xu, yu);
        if let Some(cell) = buf.cell_mut(xu, yu) {
            cell.set_symbol(glyph);
            cell.set_fg(o.color);
        }
        buf.mark_at(xu, yu, crate::render::buffer::coverage::GLYPH);
    };
    for x in left..=right {
        put(buf, x, top);
        if bottom != top {
            put(buf, x, bottom);
        }
    }
    for y in top + 1..bottom {
        put(buf, left, y);
        if right != left {
            put(buf, right, y);
        }
    }
}
