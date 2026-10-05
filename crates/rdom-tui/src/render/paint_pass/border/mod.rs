//! Border drawing: each element's border ring as per-cell ×
//! per-direction contributions (BORDER-MODEL-1), each side with its own
//! color and weight and each corner its own style, which the joiner
//! (`border_join`) turns into box-drawing glyphs. The opaque sides
//! paint in one walk of the ring (`paint_border_sides`), the sides of a
//! translucent alpha through a layer over the ring's cells.
//!
//! Clipping: border edges are painted cell by cell, each checked
//! against the `clip` rect. Negative signed coords (`LayoutRect` can be
//! negative under scroll) skip cleanly.
//!
//! - `half_block` — the inward quadrants of half-block borders.

mod half_block;

use crate::layout::{Border, BorderRadius, BorderWidth, CornerStyle, Corners, LayoutRect, Sides};
use crate::render::buffer::{BorderContribution, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W};
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};
use half_block::accumulate_half_block_quads;
use rdom_style::layout::{BorderStyle, BorderWeight};

/// Paint an element's border: the opaque sides in one walk of the
/// ring, the sides of each translucent alpha through a layer at that
/// alpha (they blend like a glyph, C3-ALPHA) over the ring's cells only,
/// a transparent side not at all — it keeps its space and draws nothing.
/// Allocates nothing (`C4G-BORDER-COST`): the per-side inputs are read
/// in place and the translucent layer is the buffer's kept scratch.
pub(super) fn paint_border_sides(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    outer_grid: Rect,
    clip: Rect,
    priority: u64,
) {
    let (border, colors) = (computed.border, computed.border_color);
    // A zero-width side is already `none` in the used border.
    let [t, r, b, l] = computed.border_width.each();
    let weight = |w: &BorderWidth| w.weight().unwrap_or_default();
    let weights = Sides::new(weight(t), weight(r), weight(b), weight(l));
    // §5.1: a corner with a non-zero radius rounds.
    let [tl, tr, br, bl] = computed.border_radius.each();
    let corner = |r: &BorderRadius| {
        if r.is_rounded(outer.width, outer.height) {
            CornerStyle::Rounded
        } else {
            CornerStyle::Square
        }
    };
    let corners = Corners::new(corner(tl), corner(tr), corner(br), corner(bl));
    // The real buffer's edges, which a direction may not point past —
    // also inside a layer, which covers only part of it.
    let bounds = buf.area;
    let pass = |colors: Sides<Color>, only: Sides<bool>, clip: Rect, buf: &mut Buffer| {
        let ink = Ink {
            colors,
            weights,
            corners,
        };
        paint_border(buf, outer, border, ink, only, clip, priority, bounds);
    };
    let opaque = colors.map(|c| c.alpha() != 0 && !c.is_translucent());
    if opaque.each().into_iter().any(|o| *o) {
        pass(colors, opaque, clip, buf);
    }
    let sides = colors.to_array();
    for (i, color) in sides.iter().enumerate() {
        let alpha = color.alpha();
        let first = !sides[..i]
            .iter()
            .any(|c| c.is_translucent() && c.alpha() == alpha);
        if !color.is_translucent() || !first {
            continue;
        }
        let only = colors.map(|c| c.is_translucent() && c.alpha() == alpha);
        let fraction = f32::from(alpha) / 255.0;
        for strip in ring_strips(outer, outer_grid, only).into_iter().flatten() {
            buf.paint_translucent(strip, fraction, |layer| {
                pass(colors.map(Color::opaque), only, strip, layer);
            });
        }
    }
}

/// The cells of the ring the sides `only` selects, as up to four
/// disjoint strips of `outer_grid` (the visible border box): the top and
/// bottom rows, and the left and right columns between them (a corner
/// joins the column when its row is not a strip — the column's side
/// draws its half).
fn ring_strips(outer: LayoutRect, outer_grid: Rect, only: Sides<bool>) -> [Option<Rect>; 4] {
    let grid = Edges::of(outer_grid);
    let (x0, y0) = (outer.x, outer.y);
    let x1 = outer.x + i32::from(outer.width) - 1;
    let y1 = outer.y + i32::from(outer.height) - 1;
    let bottom = only.bottom && outer.height >= 2;
    let right = only.right && outer.width >= 2;
    let row = |y: i32| grid.cut(grid.left, y, grid.right, y + 1);
    let col = |x: i32| {
        let top = y0 + i32::from(only.top);
        let end = y1 + 1 - i32::from(bottom);
        grid.cut(x, top, x + 1, end)
    };
    [
        if only.top { row(y0) } else { None },
        if bottom { row(y1) } else { None },
        if only.left { col(x0) } else { None },
        if right { col(x1) } else { None },
    ]
}

/// A grid rectangle by its edges, for cutting strips out of it.
struct Edges {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Edges {
    fn of(r: Rect) -> Edges {
        Edges {
            left: i32::from(r.x),
            top: i32::from(r.y),
            right: i32::from(r.right()),
            bottom: i32::from(r.bottom()),
        }
    }

    /// `(left, top)`–`(right, bottom)` inside `self`, if not empty.
    fn cut(&self, left: i32, top: i32, right: i32, bottom: i32) -> Option<Rect> {
        let (l, t) = (left.max(self.left), top.max(self.top));
        let (r, b) = (right.min(self.right), bottom.min(self.bottom));
        // Inside a grid `Rect`, so every bound fits a `u16`.
        (l < r && t < b).then(|| Rect::new(l as u16, t as u16, (r - l) as u16, (b - t) as u16))
    }
}

/// Paint the box-drawing characters for `border` along the edges of
/// `outer`, each side in its `ink`; only the sides `only`
/// selects contribute (the caller paints sides of different alpha in
/// separate passes). `bounds` is the buffer's area — a direction that
/// would point past it is dropped — given apart from `buf`, which may
/// be a layer over part of it. Writes `symbol + fg + (no modifier touch)` only — does
/// **not** touch `cell.bg`. The cell's background is owned by
/// whatever ran `fill_bg` (this element's, an ancestor's, or
/// nothing). This matches CSS: a border has its own `border-color`
/// (fg) but inherits the element's `background-color` for the cells
/// it paints over. An element with no `background-color` paints a
/// transparent border ring — the underlying cell bg shows through.
#[allow(clippy::too_many_arguments)]
fn paint_border(
    buf: &mut Buffer,
    outer: LayoutRect,
    border: Border,
    ink: Ink,
    only: Sides<bool>,
    clip: Rect,
    priority: u64,
    bounds: Rect,
) {
    // BORDER-MODEL-1: paint writes per-cell × per-direction
    // contributions to the buffer's `border_dirs`. Each
    // contribution carries the source side's `BorderStyle`, color,
    // and structural priority. The joiner (`border_join`) reads
    // every cell's per-direction state, resolves CSS Tables 3
    // §11.5 conflicts, and emits the right junction glyph + color.
    //
    // We don't `set_symbol` here at all — the joiner is the single
    // source of truth for what each border cell shows. Hidden
    // contributions kill their direction at conflict-resolution
    // time, so no extra "clear the symbol" step is needed.

    let top = border.top;
    let bot = border.bottom;
    let lft = border.left;
    let rgt = border.right;
    // `is_visible()` returns true for any non-None / non-Hidden
    // style; we still need to write a contribution for Hidden too,
    // because Hidden's job is to KILL the direction. None is the
    // only style we can short-circuit on.
    let any_top = !top.is_none();
    let any_bot = !bot.is_none();
    let any_lft = !lft.is_none();
    let any_rgt = !rgt.is_none();
    if !(any_top || any_bot || any_lft || any_rgt) {
        return;
    }

    let right_x = outer.x + outer.width as i32 - 1;
    let bottom_y = outer.y + outer.height as i32 - 1;
    let pen = Pen {
        ink,
        only,
        priority,
        corners: ink.corners,
        edges: (outer.x, outer.y, right_x, bottom_y),
    };

    // Half-block borders also accumulate their inward quadrants per cell, so
    // the joiner can weld them (union) into one outline. Solid/double borders
    // use only the per-direction model below.
    accumulate_half_block_quads(buf, outer, &border, clip, right_x, bottom_y);

    // Bit constants for the off-buffer filter — drop a direction's
    // contribution when it'd point past the viewport edge (no
    // visible neighbor to share with → no junction).
    const NM: u8 = 0b0001;
    const EM: u8 = 0b0010;
    const SM: u8 = 0b0100;
    const WM: u8 = 0b1000;

    let buf_area = bounds;
    let off_buffer = |x: u16, y: u16, bits: u8| -> u8 {
        let mut out = bits;
        if y == buf_area.y {
            out &= !NM;
        }
        if x + 1 >= buf_area.x + buf_area.width {
            out &= !EM;
        }
        if y + 1 >= buf_area.y + buf_area.height {
            out &= !SM;
        }
        if x == buf_area.x {
            out &= !WM;
        }
        out
    };

    // Top
    if any_top && in_clip_row(outer.y, clip) {
        let y = outer.y as u16;
        for x in outer.x..=right_x {
            if x < 0 {
                continue;
            }
            let xu = x as u16;
            if !clip.contains(xu, y) {
                continue;
            }
            // Top edge contributes E (going east) and W (going west)
            // to interior cells; the corner cells additionally get
            // S contributions from the LEFT or RIGHT border.
            let bits = if any_lft && x == outer.x {
                EM | SM
            } else if any_rgt && x == right_x {
                WM | SM
            } else {
                EM | WM
            };
            let allowed = off_buffer(xu, y, bits);
            // Horizontal segments (E / W) carry `top`'s style.
            if allowed & EM != 0 {
                pen.add(buf, xu, y, DIR_E, top, BorderSide::Top);
            }
            if allowed & WM != 0 {
                pen.add(buf, xu, y, DIR_W, top, BorderSide::Top);
            }
            // The S segment at a top corner carries the LEFT/RIGHT
            // border's style — it's the start of the vertical line.
            if allowed & SM != 0 {
                let (style, src_side) = if x == outer.x {
                    (lft, BorderSide::Left)
                } else {
                    (rgt, BorderSide::Right)
                };
                pen.add(buf, xu, y, DIR_S, style, src_side);
            }
        }
    }

    // Bottom
    if any_bot && in_clip_row(bottom_y, clip) && outer.height >= 2 {
        let y = bottom_y as u16;
        for x in outer.x..=right_x {
            if x < 0 {
                continue;
            }
            let xu = x as u16;
            if !clip.contains(xu, y) {
                continue;
            }
            let bits = if any_lft && x == outer.x {
                NM | EM
            } else if any_rgt && x == right_x {
                NM | WM
            } else {
                EM | WM
            };
            let allowed = off_buffer(xu, y, bits);
            if allowed & EM != 0 {
                pen.add(buf, xu, y, DIR_E, bot, BorderSide::Bottom);
            }
            if allowed & WM != 0 {
                pen.add(buf, xu, y, DIR_W, bot, BorderSide::Bottom);
            }
            if allowed & NM != 0 {
                let (style, src_side) = if x == outer.x {
                    (lft, BorderSide::Left)
                } else {
                    (rgt, BorderSide::Right)
                };
                pen.add(buf, xu, y, DIR_N, style, src_side);
            }
        }
    }

    // Left (skip cells already covered by corners — those wrote
    // their N/S contributions above)
    if any_lft && in_clip_col(outer.x, clip) {
        let x = outer.x as u16;
        let y_start = if any_top { outer.y + 1 } else { outer.y };
        let y_end = if any_bot { bottom_y - 1 } else { bottom_y };
        for y in y_start..=y_end {
            if y < 0 {
                continue;
            }
            let yu = y as u16;
            if !clip.contains(x, yu) {
                continue;
            }
            let allowed = off_buffer(x, yu, NM | SM);
            if allowed & NM != 0 {
                pen.add(buf, x, yu, DIR_N, lft, BorderSide::Left);
            }
            if allowed & SM != 0 {
                pen.add(buf, x, yu, DIR_S, lft, BorderSide::Left);
            }
        }
    }

    // Right
    if any_rgt && in_clip_col(right_x, clip) && outer.width >= 2 {
        let x = right_x as u16;
        let y_start = if any_top { outer.y + 1 } else { outer.y };
        let y_end = if any_bot { bottom_y - 1 } else { bottom_y };
        for y in y_start..=y_end {
            if y < 0 {
                continue;
            }
            let yu = y as u16;
            if !clip.contains(x, yu) {
                continue;
            }
            let allowed = off_buffer(x, yu, NM | SM);
            if allowed & NM != 0 {
                pen.add(buf, x, yu, DIR_N, rgt, BorderSide::Right);
            }
            if allowed & SM != 0 {
                pen.add(buf, x, yu, DIR_S, rgt, BorderSide::Right);
            }
        }
    }
}

/// Each side's `border-*-color` and line weight (from its
/// `border-*-width`), and each corner's glyph (from its
/// `border-*-radius`).
#[derive(Clone, Copy)]
struct Ink {
    colors: Sides<Color>,
    weights: Sides<BorderWeight>,
    corners: Corners<CornerStyle>,
}

/// What one element's border contributes with: each side's ink and
/// whether this pass paints it, the structural priority, and each
/// corner's style with where the corners are (left, top, right, bottom).
struct Pen {
    ink: Ink,
    only: Sides<bool>,
    priority: u64,
    corners: Corners<CornerStyle>,
    edges: (i32, i32, i32, i32),
}

impl Pen {
    /// Add one direction's contribution to the cell. Routes the
    /// source side's `BorderStyle` + `border-*-color` + structural
    /// priority through `add_border_dir` so CSS Tables 3 §11.5
    /// conflict resolution can decide the winner per direction.
    #[inline]
    fn add(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        dir: usize,
        style: BorderStyle,
        side: BorderSide,
    ) {
        let (colors, weights, only) = (&self.ink.colors, &self.ink.weights, &self.only);
        let (fg, weight, painted) = match side {
            BorderSide::Top => (colors.top, weights.top, only.top),
            BorderSide::Right => (colors.right, weights.right, only.right),
            BorderSide::Bottom => (colors.bottom, weights.bottom, only.bottom),
            BorderSide::Left => (colors.left, weights.left, only.left),
        };
        if style.is_none() || !painted {
            return;
        }
        buf.add_border_dir(
            x,
            y,
            dir,
            BorderContribution {
                style,
                fg,
                weight,
                priority: self.priority,
                corner_style: self.corner_at(x, y),
                side,
            },
        );
    }
}

impl Pen {
    /// The corner style of the cell `(x, y)`: its corner's when it is
    /// one of the box's four corners, square elsewhere.
    fn corner_at(&self, x: u16, y: u16) -> CornerStyle {
        let (x, y) = (i32::from(x), i32::from(y));
        let (left, top, right, bottom) = self.edges;
        match (x == left, x == right, y == top, y == bottom) {
            (true, _, true, _) => self.corners.top_left,
            (_, true, true, _) => self.corners.top_right,
            (_, true, _, true) => self.corners.bottom_right,
            (true, _, _, true) => self.corners.bottom_left,
            _ => CornerStyle::Square,
        }
    }
}

#[inline]
fn in_clip_row(y_signed: i32, clip: Rect) -> bool {
    y_signed >= clip.y as i32 && y_signed < clip.bottom() as i32
}

#[inline]
fn in_clip_col(x_signed: i32, clip: Rect) -> bool {
    x_signed >= clip.x as i32 && x_signed < clip.right() as i32
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
