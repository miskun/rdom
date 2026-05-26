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
use crate::render::{Buffer, Modifier, Rect, Style};
use crate::style::Color;

// ─── Box-drawing characters ─────────────────────────────────────────

const SINGLE_TL: &str = "┌";
const SINGLE_TR: &str = "┐";
const SINGLE_BL: &str = "└";
const SINGLE_BR: &str = "┘";
const ROUNDED_TL: &str = "╭";
const ROUNDED_TR: &str = "╮";
const ROUNDED_BL: &str = "╰";
const ROUNDED_BR: &str = "╯";
const HORIZONTAL: &str = "─";
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
) {
    // Per the project's paint-layer invariant: only `fill_bg` writes
    // `cell.bg`. Glyph painters write `symbol + fg + modifiers` only.
    // Building a `Style` with `.bg(...)` here — even with
    // `Color::Reset` — would wipe the underlying bg via
    // `Cell::apply_style`'s `Some(Color::Reset)` write.
    let style = Style::new().fg(border_fg);

    // Which edges are drawn?
    let top_edge = border.top;
    let bottom_edge = border.bottom;
    let left_edge = border.left;
    let right_edge = border.right;

    let (tl, tr, bl, br) = match border.corner_style {
        crate::layout::CornerStyle::Rounded => (ROUNDED_TL, ROUNDED_TR, ROUNDED_BL, ROUNDED_BR),
        crate::layout::CornerStyle::Square => (SINGLE_TL, SINGLE_TR, SINGLE_BL, SINGLE_BR),
    };

    let right_x = outer.x + outer.width as i32 - 1;
    let bottom_y = outer.y + outer.height as i32 - 1;

    // Per-direction bits for the additive border mask (M5-COLLAPSE-2
    // fix). When two rings' cells coincide, masks OR; the joiner
    // reads the final mask to derive the right junction glyph.
    const N: u8 = 0b0001;
    const E: u8 = 0b0010;
    const S: u8 = 0b0100;
    const W: u8 = 0b1000;

    // Drop bits whose adjacent cell is outside the buffer. Lines
    // that logically continue off-screen leave no visible
    // counterpart there, so a bit pointing off-buffer would yield
    // a stray glyph during joiner accumulation (e.g. a `┼` where
    // a `┤` was correct because an overflowing top border kept
    // its E bit at the rightmost visible cell). Filtering at the
    // paint site keeps the joiner pure — read mask, look up
    // glyph, no viewport-edge logic — and reflects the substrate
    // truth that the mask only encodes visible connectivity.
    let buf_area = buf.area;
    let filter_off_buffer = |x: u16, y: u16, bits: u8| -> u8 {
        let mut out = bits;
        if y == buf_area.y {
            out &= !N;
        }
        if x + 1 >= buf_area.x + buf_area.width {
            out &= !E;
        }
        if y + 1 >= buf_area.y + buf_area.height {
            out &= !S;
        }
        if x == buf_area.x {
            out &= !W;
        }
        out
    };

    // Top
    if top_edge && in_clip_row(outer.y, clip) {
        let y = outer.y as u16;
        for x in outer.x..=right_x {
            if x < 0 {
                continue;
            }
            let xu = x as u16;
            if !clip.contains(xu, y) {
                continue;
            }
            // Top edge has a line going horizontally (E + W) PLUS,
            // at the corners, a line going down (S). The corner
            // glyph itself encodes (E + S) at TL and (W + S) at TR.
            let (symbol, bits) = if left_edge && x == outer.x {
                (tl, E | S)
            } else if right_edge && x == right_x {
                (tr, W | S)
            } else {
                (HORIZONTAL, E | W)
            };
            buf.set_symbol(xu, y, symbol, style);
            buf.add_border_mask(xu, y, filter_off_buffer(xu, y, bits));
        }
    }

    // Bottom
    if bottom_edge && in_clip_row(bottom_y, clip) && outer.height >= 2 {
        let y = bottom_y as u16;
        for x in outer.x..=right_x {
            if x < 0 {
                continue;
            }
            let xu = x as u16;
            if !clip.contains(xu, y) {
                continue;
            }
            let (symbol, bits) = if left_edge && x == outer.x {
                (bl, N | E)
            } else if right_edge && x == right_x {
                (br, N | W)
            } else {
                (HORIZONTAL, E | W)
            };
            buf.set_symbol(xu, y, symbol, style);
            buf.add_border_mask(xu, y, filter_off_buffer(xu, y, bits));
        }
    }

    // Left (skip cells already filled by corners)
    if left_edge && in_clip_col(outer.x, clip) {
        let x = outer.x as u16;
        let y_start = if top_edge { outer.y + 1 } else { outer.y };
        let y_end = if bottom_edge { bottom_y - 1 } else { bottom_y };
        for y in y_start..=y_end {
            if y < 0 {
                continue;
            }
            let yu = y as u16;
            if !clip.contains(x, yu) {
                continue;
            }
            buf.set_symbol(x, yu, VERTICAL, style);
            buf.add_border_mask(x, yu, filter_off_buffer(x, yu, N | S));
        }
    }

    // Right
    if right_edge && in_clip_col(right_x, clip) && outer.width >= 2 {
        let x = right_x as u16;
        let y_start = if top_edge { outer.y + 1 } else { outer.y };
        let y_end = if bottom_edge { bottom_y - 1 } else { bottom_y };
        for y in y_start..=y_end {
            if y < 0 {
                continue;
            }
            let yu = y as u16;
            if !clip.contains(x, yu) {
                continue;
            }
            buf.set_symbol(x, yu, VERTICAL, style);
            buf.add_border_mask(x, yu, filter_off_buffer(x, yu, N | S));
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
