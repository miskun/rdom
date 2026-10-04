//! Half-block borders: each cell of the ring records the quadrants
//! the border fills inward, which the joiner unions across elements to
//! weld adjacent half-block borders into one outline.

use crate::layout::{Border, LayoutRect};
use crate::render::{Buffer, Rect};
use rdom_style::layout::BorderStyle;

/// Accumulate this element's inward half-block quadrants into the buffer for
/// every cell on its border ring. A cell's inward region is the intersection
/// of its vertical half (top edge → bottom half, bottom edge → top half, else
/// full height) and its horizontal half (left edge → right half, right edge →
/// left half, else full width) — so a plain edge fills a half and a corner
/// fills a single quadrant. Only half-block sides contribute; the joiner unions
/// these across elements to weld adjacent half-block borders.
pub(super) fn accumulate_half_block_quads(
    buf: &mut Buffer,
    outer: LayoutRect,
    border: &Border,
    clip: Rect,
    right_x: i32,
    bottom_y: i32,
) {
    use crate::render::buffer::{QUAD_BL, QUAD_BR, QUAD_TL, QUAD_TR};
    let hb = |s: BorderStyle| s == BorderStyle::HalfBlock;
    let (t, b, l, r) = (
        hb(border.top),
        hb(border.bottom),
        hb(border.left),
        hb(border.right),
    );
    if !(t || b || l || r) {
        return;
    }
    let top_q = QUAD_TL | QUAD_TR;
    let bot_q = QUAD_BL | QUAD_BR;
    let left_q = QUAD_TL | QUAD_BL;
    let right_q = QUAD_TR | QUAD_BR;
    let full = top_q | bot_q;

    let visit = |buf: &mut Buffer, x: i32, y: i32| {
        if x < 0 || y < 0 {
            return;
        }
        let (xu, yu) = (x as u16, y as u16);
        if !clip.contains(xu, yu) {
            return;
        }
        let is_top = y == outer.y && t;
        let is_bot = y == bottom_y && b;
        let is_lft = x == outer.x && l;
        let is_rgt = x == right_x && r;
        // Vertical half this cell fills, then horizontal half; the inward
        // quadrant set is their intersection.
        let vert = match (is_top, is_bot) {
            (true, false) => bot_q,
            (false, true) => top_q,
            _ => full,
        };
        let horiz = match (is_lft, is_rgt) {
            (true, false) => right_q,
            (false, true) => left_q,
            _ => full,
        };
        let quads = vert & horiz;
        // A cell with no half-block side passing through contributes nothing.
        if is_top || is_bot || is_lft || is_rgt {
            buf.add_half_block_quads(xu, yu, quads);
        }
    };

    for x in outer.x..=right_x {
        visit(buf, x, outer.y);
        if bottom_y != outer.y {
            visit(buf, x, bottom_y);
        }
    }
    for y in outer.y..=bottom_y {
        visit(buf, outer.x, y);
        if right_x != outer.x {
            visit(buf, right_x, y);
        }
    }
}
