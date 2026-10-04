//! Border drawing: each element's border ring as per-cell ×
//! per-direction contributions (BORDER-MODEL-1), which the joiner
//! (`border_join`) turns into box-drawing glyphs.
//!
//! Clipping: border edges are painted cell by cell, each checked
//! against the `clip` rect. Negative signed coords (`LayoutRect` can be
//! negative under scroll) skip cleanly.
//!
//! - `half_block` — the inward quadrants of half-block borders.

mod half_block;

use crate::layout::{Border, LayoutRect, Sides};
use crate::render::buffer::{BorderContribution, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W};
use crate::render::{Buffer, Rect};
use crate::style::Color;
use half_block::accumulate_half_block_quads;
use rdom_style::layout::{BorderStyle, BorderWeight};

/// Paint the box-drawing characters for `border` along the edges of
/// `outer`, each side in its `ink`; only the sides `only`
/// selects contribute (the caller paints sides of different alpha in
/// separate passes). Writes `symbol + fg + (no modifier touch)` only — does
/// **not** touch `cell.bg`. The cell's background is owned by
/// whatever ran `fill_bg` (this element's, an ancestor's, or
/// nothing). This matches CSS: a border has its own `border-color`
/// (fg) but inherits the element's `background-color` for the cells
/// it paints over. An element with no `background-color` paints a
/// transparent border ring — the underlying cell bg shows through.
pub(super) fn paint_border(
    buf: &mut Buffer,
    outer: LayoutRect,
    border: Border,
    ink: Ink,
    only: Sides<bool>,
    clip: Rect,
    priority: u64,
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

    let pen = Pen {
        ink,
        only,
        priority,
        corner_style: border.corner_style,
    };
    let right_x = outer.x + outer.width as i32 - 1;
    let bottom_y = outer.y + outer.height as i32 - 1;

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

    let buf_area = buf.area;
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
/// `border-*-width`).
#[derive(Clone, Copy)]
pub(super) struct Ink {
    pub colors: Sides<Color>,
    pub weights: Sides<BorderWeight>,
}

/// What one element's border contributes with: each side's ink and
/// whether this pass paints it, the structural priority and the corner
/// style.
struct Pen {
    ink: Ink,
    only: Sides<bool>,
    priority: u64,
    corner_style: crate::layout::CornerStyle,
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
                corner_style: self.corner_style,
                side,
            },
        );
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
