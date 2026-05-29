//! Background fill + border drawing.
//!
//! Box-drawing characters live here (constants) to keep the shape
//! lookup tight. `Border::single()` uses square corners; `Rounded`
//! uses arc corners; `Top/Bottom/Left/Right` draw just that one
//! edge (no corners needed).
//!
//! Clipping: border edges are painted grapheme-by-grapheme, each
//! checked against the `clip` rect. Negative signed coords
//! (`LayoutRect` can be negative under scroll) skip cleanly.

use crate::layout::{Border, LayoutRect};
use crate::render::buffer::{BorderContribution, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W};
use crate::render::{Buffer, Modifier, Rect, Style};
use crate::style::Color;
use rdom_style::layout::BorderStyle;

// ─── Box-drawing characters ─────────────────────────────────────────
//
// The actual glyphs are emitted by the joiner (`border_join.rs`)
// from the per-direction state this module writes. Paint here
// is now structural — it stamps `(BorderStyle, fg, priority)`
// per cell × direction; the joiner reconciles conflicts and picks
// the right glyph. These constants are kept for any future
// non-collapse fast path / fallback.

#[allow(dead_code)]
const SINGLE_TL: &str = "┌";
#[allow(dead_code)]
const SINGLE_TR: &str = "┐";
#[allow(dead_code)]
const SINGLE_BL: &str = "└";
#[allow(dead_code)]
const SINGLE_BR: &str = "┘";
#[allow(dead_code)]
const ROUNDED_TL: &str = "╭";
#[allow(dead_code)]
const ROUNDED_TR: &str = "╮";
#[allow(dead_code)]
const ROUNDED_BL: &str = "╰";
#[allow(dead_code)]
const ROUNDED_BR: &str = "╯";
#[allow(dead_code)]
const HORIZONTAL: &str = "─";
#[allow(dead_code)]
const VERTICAL: &str = "│";

// ─── Background ─────────────────────────────────────────────────────

/// Fill `area` cells with `bg`. Behavior depends on the painter's
/// effective `opacity` — the project's three-regime compositing
/// rule:
///
/// - **Opaque** (`opacity >= 1.0`) — full CSS opaque box. Writes
///   `cell.bg = bg` AND **clears `cell.symbol` to SPACE**, clears
///   `cell.fg` to `Color::Reset`, and clears `cell.modifier`. Any
///   glyph an earlier paint deposited in this cell is replaced by
///   a blank canvas, ready for this element's own border / text /
///   pseudo content to paint over it. Without this clear, glyphs
///   from lower z-layers (or earlier tree-order paints) leak
///   through an opaque overlay's bg — the visible bug in
///   `positioning_demo` before 2026-05-18.
///
/// - **Translucent** (`0.0 < opacity < 1.0`) — sets `cell.bg`
///   only, preserving the cell's existing `symbol`, `fg`, and
///   `modifier`. Underlying glyphs bleed through, tinted by the
///   blended bg (the painter's bg is alpha-blended against
///   `parent_bg` at cascade time before reaching `fill_bg`). This
///   is what CSS authors expect from `opacity < 1`: the layer is
///   semi-transparent, content underneath shows through.
///
/// - **Invisible** (`opacity <= 0.0`) — caller is responsible for
///   skipping the call. We don't gate here because `alpha_blend`
///   in `mod.rs` already collapses fg/bg to `parent_bg` for
///   `opacity = 0`, so calling `fill_bg` with that collapsed bg
///   is a no-op visually; gating would be a small optimization.
///
/// `Color::Reset` for `bg` is honored at the call site (caller
/// gates `if computed.bg != Color::Reset`); this function assumes
/// the caller has decided to paint.
///
/// Wide-glyph handling: when the opaque clear writes a SPACE over
/// a wide-glyph primary cell, the partner spacer at `x+1` is also
/// cleared (and vice versa for spacer cells). Without this pairing,
/// the fill would leave half-cell residue at area boundaries that
/// straddle a wide glyph.
pub(super) fn fill_bg(buf: &mut Buffer, area: Rect, bg: Color, opacity: f32) {
    let opaque = opacity >= 1.0;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if opaque {
                // Opaque fast path: clear symbol/fg/modifier so the
                // element's subsequent border/text/pseudo paints land
                // on a blank canvas, then write the raw bg.
                clear_cell_for_opaque_fill(buf, x, y);
                // BORDER-MODEL-1: an opaque fill completely occludes
                // anything painted at this cell earlier in the walk,
                // including border contributions from underlying
                // elements. Clear the per-direction state so the
                // joiner doesn't re-emit a border glyph here.
                clear_border_dirs(buf, x, y);
                if let Some(cell) = buf.cell_mut(x, y) {
                    cell.bg = bg;
                }
            } else {
                // Translucent: blend the painter's `bg` against the
                // cell's existing `bg` at the current compose alpha
                // (Phase 2 cell-level RMW). When the cell is `Reset`
                // the compose context falls back to its
                // `parent_bg`, then to the `#000000` canvas model.
                // Note: `compose_bg_for_cell` honors the buffer's
                // compose context, which `paint_node` has set to
                // this element's `opacity` — the local `opacity`
                // arg matches.
                let blended = buf.compose_bg_for_cell(x, y, bg);
                if let Some(cell) = buf.cell_mut(x, y) {
                    cell.bg = blended;
                }
            }
        }
    }
}

/// Clear the per-direction border state at `(x, y)`. Called by
/// the opaque-`fill_bg` fast path so subsequent joiner runs don't
/// resurrect border glyphs that the opaque fill should occlude.
fn clear_border_dirs(buf: &mut Buffer, x: u16, y: u16) {
    use crate::render::buffer::BorderDirState;
    for dir in 0..4 {
        buf.set_border_dir(x, y, dir, BorderDirState::default());
    }
}

/// Clear `(x, y)` to a blank cell suitable for opaque-bg overpainting:
/// SPACE symbol, `fg = Reset`, no modifier. If the cell is the primary
/// of a wide glyph, also clear the trailing spacer at `x+1`; if it's a
/// spacer, also clear the primary at `x-1`. Without this pairing, an
/// area whose edge cuts a wide glyph would leave a half-cell residue.
fn clear_cell_for_opaque_fill(buf: &mut Buffer, x: u16, y: u16) {
    let (also_clear_x, primary_side): (Option<u16>, _) = match buf.cell(x, y) {
        Some(c) if c.is_spacer() => (x.checked_sub(1), "spacer's primary"),
        Some(c) if c.cell_width() == 2 => (Some(x.saturating_add(1)), "wide-glyph spacer"),
        _ => (None, ""),
    };
    let _ = primary_side; // documentation hint only
    if let Some(partner_x) = also_clear_x
        && let Some(partner) = buf.cell_mut(partner_x, y)
    {
        partner.set_symbol(" ");
        partner.fg = Color::Reset;
        partner.modifier = Modifier::empty();
    }
    if let Some(cell) = buf.cell_mut(x, y) {
        cell.set_symbol(" ");
        cell.fg = Color::Reset;
        cell.modifier = Modifier::empty();
    }
}

// ─── Border ─────────────────────────────────────────────────────────

/// Paint the box-drawing characters for `border` along the edges of
/// `outer`. Writes `symbol + fg + (no modifier touch)` only — does
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
    border_fg: Color,
    clip: Rect,
    priority: u64,
) {
    // BORDER-MODEL-1: paint writes per-cell × per-direction
    // contributions to the buffer's `border_dirs`. Each
    // contribution carries the source side's `BorderStyle`, color,
    // and structural priority. The joiner (`border_join.rs`) reads
    // every cell's per-direction state, resolves CSS Tables 3
    // §11.5 conflicts, and emits the right junction glyph + color.
    //
    // We don't `set_symbol` here at all — the joiner is the single
    // source of truth for what each border cell shows. Hidden
    // contributions kill their direction at conflict-resolution
    // time, so no extra "clear the symbol" step is needed.

    // `Style::new()` is no longer used here — the joiner is the
    // single source of truth for symbol+fg writes at border cells.
    let _ = Style::new();

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
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_E,
                    top,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Top,
                );
            }
            if allowed & WM != 0 {
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_W,
                    top,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Top,
                );
            }
            // The S segment at a top corner carries the LEFT/RIGHT
            // border's style — it's the start of the vertical line.
            if allowed & SM != 0 {
                let (style, src_side) = if x == outer.x {
                    (lft, BorderSide::Left)
                } else {
                    (rgt, BorderSide::Right)
                };
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_S,
                    style,
                    border_fg,
                    priority,
                    border.corner_style,
                    src_side,
                );
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
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_E,
                    bot,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Bottom,
                );
            }
            if allowed & WM != 0 {
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_W,
                    bot,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Bottom,
                );
            }
            if allowed & NM != 0 {
                let (style, src_side) = if x == outer.x {
                    (lft, BorderSide::Left)
                } else {
                    (rgt, BorderSide::Right)
                };
                add_dir(
                    buf,
                    xu,
                    y,
                    DIR_N,
                    style,
                    border_fg,
                    priority,
                    border.corner_style,
                    src_side,
                );
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
                add_dir(
                    buf,
                    x,
                    yu,
                    DIR_N,
                    lft,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Left,
                );
            }
            if allowed & SM != 0 {
                add_dir(
                    buf,
                    x,
                    yu,
                    DIR_S,
                    lft,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Left,
                );
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
                add_dir(
                    buf,
                    x,
                    yu,
                    DIR_N,
                    rgt,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Right,
                );
            }
            if allowed & SM != 0 {
                add_dir(
                    buf,
                    x,
                    yu,
                    DIR_S,
                    rgt,
                    border_fg,
                    priority,
                    border.corner_style,
                    BorderSide::Right,
                );
            }
        }
    }
}

/// Add one direction's contribution to the cell. Routes the
/// element's per-side `BorderStyle` + `border-color` + structural
/// priority through `add_border_dir` so CSS Tables 3 §11.5
/// conflict resolution can decide the winner per direction.
#[inline]
#[allow(clippy::too_many_arguments)]
fn add_dir(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    dir: usize,
    style: BorderStyle,
    fg: Color,
    priority: u64,
    corner_style: crate::layout::CornerStyle,
    side: BorderSide,
) {
    if style.is_none() {
        return;
    }
    buf.add_border_dir(
        x,
        y,
        dir,
        BorderContribution {
            style,
            fg,
            priority,
            corner_style,
            side,
        },
    );
}

#[inline]
fn in_clip_row(y_signed: i32, clip: Rect) -> bool {
    y_signed >= clip.y as i32 && y_signed < clip.bottom() as i32
}

#[inline]
fn in_clip_col(x_signed: i32, clip: Rect) -> bool {
    x_signed >= clip.x as i32 && x_signed < clip.right() as i32
}
