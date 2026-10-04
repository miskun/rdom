//! `box-shadow` (CSS Backgrounds 3 §6.1): each shadow a shade of whole
//! cells in its color. An outer shadow is the border box offset by the
//! shadow's offsets and grown by its spread, drawn outside the border
//! box only, under the background; an `inset` shadow is the padding box
//! less itself offset and shrunk, drawn above the background. Offsets
//! and spread are whole cells (`PaintLength::offset_cells`); the blur
//! has no effect (DIVERGENCES §2). The first shadow is on top.

use super::background::fill_bg;
use super::layout_rect_to_grid;
use crate::layout::{BoxShadow, LayoutRect, compute_padding_box};
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

/// Paint `computed`'s outer shadows around the border box `outer`.
pub(super) fn paint_outer_shadows(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
) {
    for s in computed.box_shadow.iter().rev().filter(|s| !s.inset) {
        let spread = s.spread.offset_cells();
        let shade = grow(offset(outer, s), spread);
        for part in minus(shade, outer) {
            fill(buf, part, s.color, clip);
        }
    }
}

/// Paint `computed`'s inset shadows inside the padding box of the
/// border box `outer`.
pub(super) fn paint_inset_shadows(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
) {
    let padding_box = compute_padding_box(outer, computed.border);
    for s in computed.box_shadow.iter().rev().filter(|s| s.inset) {
        let spread = s.spread.offset_cells();
        let hole = grow(offset(padding_box, s), -spread);
        for part in minus(padding_box, hole) {
            fill(buf, part, s.color, clip);
        }
    }
}

/// `r` moved by the shadow's offsets.
fn offset(r: LayoutRect, s: &BoxShadow<Color>) -> LayoutRect {
    LayoutRect::new(
        r.x.saturating_add(s.offset_x.offset_cells()),
        r.y.saturating_add(s.offset_y.offset_cells()),
        r.width,
        r.height,
    )
}

/// `r` grown by `by` cells on every side (shrunk when negative, to
/// nothing at most).
fn grow(r: LayoutRect, by: i32) -> LayoutRect {
    let size = |extent: u16| (i32::from(extent) + 2 * by).clamp(0, i32::from(u16::MAX)) as u16;
    LayoutRect::new(r.x - by, r.y - by, size(r.width), size(r.height))
}

/// The parts of `a` outside `b`: up to four bands.
fn minus(a: LayoutRect, b: LayoutRect) -> Vec<LayoutRect> {
    let (ar, ab) = (a.x + i32::from(a.width), a.y + i32::from(a.height));
    let (br, bb) = (b.x + i32::from(b.width), b.y + i32::from(b.height));
    let (ix, iy, ir, ib) = (a.x.max(b.x), a.y.max(b.y), ar.min(br), ab.min(bb));
    if ix >= ir || iy >= ib {
        return vec![a];
    }
    let rect = |x: i32, y: i32, r: i32, b: i32| {
        (r > x && b > y).then(|| LayoutRect::new(x, y, (r - x) as u16, (b - y) as u16))
    };
    [
        rect(a.x, a.y, ar, iy),
        rect(a.x, ib, ar, ab),
        rect(a.x, iy, ix, ib),
        rect(ir, iy, ar, ib),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Fill `part` with `color` as an opaque shade over what is beneath, or
/// a translucent one composited over it (C3-ALPHA).
fn fill(buf: &mut Buffer, part: LayoutRect, color: Color, clip: Rect) {
    if !super::fills(color) {
        return;
    }
    let Some(area) = layout_rect_to_grid(part, clip) else {
        return;
    };
    if color.is_translucent() {
        let alpha = f32::from(color.alpha()) / 255.0;
        buf.paint_translucent(area, alpha, |layer| fill_bg(layer, area, color.opaque()));
    } else {
        fill_bg(buf, area, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parts of a rect outside another cover exactly the cells of
    /// the first not in the second.
    #[test]
    fn minus_leaves_the_cells_outside() {
        let a = LayoutRect::new(0, 0, 4, 3);
        let b = LayoutRect::new(1, 1, 2, 5);
        let parts = minus(a, b);
        let mut cells: Vec<(i32, i32)> = parts
            .iter()
            .flat_map(|r| {
                (r.y..r.y + i32::from(r.height))
                    .flat_map(move |y| (r.x..r.x + i32::from(r.width)).map(move |x| (x, y)))
            })
            .collect();
        cells.sort();
        let mut expected: Vec<(i32, i32)> = (0..3)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| !(1..3).contains(&x) || y < 1)
            .collect();
        expected.sort();
        assert_eq!(cells, expected);
        assert_eq!(minus(a, LayoutRect::new(9, 9, 1, 1)), vec![a]);
    }
}
