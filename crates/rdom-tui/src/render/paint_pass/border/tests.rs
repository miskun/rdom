//! The border paint's cost (`C4G-BORDER-COST`): no allocation per
//! bordered element per frame, whatever its sides' colors and radii.

use super::*;
use crate::layout::{BorderRadius, BorderWidth, PaintLength};
use crate::test_alloc::allocations_in;

/// A 6 × 4 ring at (1, 1) in an 8 × 6 buffer, four side colors — one
/// translucent — and a radius that is a math expression (a boxed
/// `PaintLength::Calc`, which a clone would copy).
fn bordered() -> (Buffer, ComputedStyle, LayoutRect, Rect) {
    let buf = Buffer::empty(Rect::new(0, 0, 8, 6));
    let mut c = ComputedStyle::initial();
    c.border = Border::single();
    c.border_color = Sides::new(
        Color::Rgb(255, 0, 0),
        Color::Rgb(0, 255, 0),
        Color::Rgb(0, 0, 255),
        Color::Rgba(255, 255, 255, 128),
    );
    c.border_width = Sides::all(BorderWidth::Thick);
    let calc = crate::calc::CalcExpr::Length(1);
    c.border_radius = Corners::all(BorderRadius::circle(PaintLength::Calc(Box::new(calc))));
    (buf, c, LayoutRect::new(1, 1, 6, 4), Rect::new(1, 1, 6, 4))
}

/// Painting a bordered element allocates nothing once the buffer's
/// translucent scratch layer is sized (it is kept across frames).
#[test]
fn a_border_paints_without_allocating() {
    let (mut buf, c, outer, grid) = bordered();
    let clip = buf.area;
    // Warm-up: the first translucent paint sizes the scratch layer.
    paint_border_sides(&mut buf, &c, outer, grid, clip, 7);
    let n = allocations_in(|| paint_border_sides(&mut buf, &c, outer, grid, clip, 7));
    assert_eq!(n, 0, "allocations painting one bordered element");
}
