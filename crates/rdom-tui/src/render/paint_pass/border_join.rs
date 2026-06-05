//! Border join pass — single source of truth for border glyphs +
//! colors. Runs after the main paint pass.
//!
//! `paint_border` records per-cell × per-direction contributions in
//! `buf.border_dirs`. Each contribution carries a [`BorderStyle`],
//! a foreground color, and a structural priority. Inside each
//! direction the contributions are reconciled per CSS Tables 3
//! §11.5: `hidden` kills the direction; otherwise the
//! highest-rank, highest-priority contribution wins.
//!
//! The joiner walks the buffer once and, for every cell that has
//! at least one visible direction, emits the junction glyph + the
//! winning direction's foreground color. The glyph is chosen from
//! one of two 16-entry tables depending on the cell's dominant
//! style:
//!
//! - `BorderStyle::Double` → double-line glyphs (`║═╔╗╚╝╠╣╦╩╬`).
//! - Anything else (`Solid`, `Dashed`, `Dotted`, `Ridge`, `Outset`,
//!   `Groove`, `Inset`) → single-line glyphs (`│─┌┐└┘├┤┬┴┼`).
//!   The non-solid keywords parse and rank correctly in conflict
//!   resolution but degrade to the single-line glyph set on the
//!   terminal — CSS-faithful "render as best you can" per the
//!   medium constraint documented in `DIVERGENCES.md`.
//!
//! BORDER-MODEL-1 retires the previous `tree_has_collapse` gate.
//! Conflict resolution is now per-direction and runs whenever any
//! border contribution exists at a cell, regardless of whether the
//! ancestor declared collapse. The cost is one buffer-wide sweep;
//! cells with no border contribution short-circuit on the
//! per-cell `is_visible()` test.
//!
//! **Paint-layer invariant preserved:** reads + writes
//! `cell.symbol` and `cell.fg` only. Never touches `cell.bg`.

use rdom_core::Dom;
use rdom_style::layout::{BorderStyle, CornerStyle};

use crate::ext::TuiExt;
use crate::render::Buffer;
use crate::render::buffer::{
    BorderContribution, BorderDirState, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W,
};

pub(super) fn join_borders(_dom: &Dom<TuiExt>, buf: &mut Buffer) {
    let area = buf.area;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            // Half-block borders are resolved by inward-quadrant union, not the
            // single-line direction tables. A non-zero accumulator means at
            // least one half-block border passes through this cell; emit the
            // welded block glyph and move on. Color comes from the dominant
            // direction contribution (the half-block side also records one).
            let quads = buf.half_block_quads_at(x, y);
            if quads != 0 {
                let glyph = half_block_quad_glyph(quads);
                if !glyph.is_empty() {
                    let cell_state = [
                        buf.border_dir_at(x, y, DIR_N),
                        buf.border_dir_at(x, y, DIR_E),
                        buf.border_dir_at(x, y, DIR_S),
                        buf.border_dir_at(x, y, DIR_W),
                    ];
                    let fg = dominant_contribution(&cell_state).fg;
                    if let Some(cell) = buf.cell_mut(x, y) {
                        cell.set_symbol(glyph);
                        if fg != crate::style::Color::Reset {
                            cell.set_fg(fg);
                        }
                    }
                }
                continue;
            }
            let cell_state = [
                buf.border_dir_at(x, y, DIR_N),
                buf.border_dir_at(x, y, DIR_E),
                buf.border_dir_at(x, y, DIR_S),
                buf.border_dir_at(x, y, DIR_W),
            ];
            let mask = visible_mask(&cell_state);
            if mask == 0 {
                continue;
            }
            let dominant = dominant_contribution(&cell_state);
            // Rounded-corner fast path: when this cell is a lone
            // bordered element's corner — exactly one priority
            // contributes across the visible directions, mask is
            // a corner pattern (E+S / W+S / N+E / N+W), and the
            // contribution declared `CornerStyle::Rounded` — emit
            // the rounded glyph instead of the square one. Any
            // overlap (multiple priorities) → square junction,
            // because Unicode has no rounded T-junctions.
            let lone = is_lone_contributor(&cell_state, dominant.priority);
            // Half-block borders never reach here — they're resolved above by
            // the inward-quadrant union, which subsumes the old (mask, side)
            // lone-element table AND adds T-junctions / welds. The quadrant
            // glyph leaves the cell's bg ALONE, so each glyph's "empty"
            // quadrants merge into whatever the parent painted — rdom's analog
            // of CSS `background-clip: padding-box` (see DIVERGENCES.md).
            if lone
                && dominant.corner_style == CornerStyle::Rounded
                && dominant.style != BorderStyle::Double
            {
                let rounded = ROUNDED_TABLE[mask as usize];
                if !rounded.is_empty()
                    && let Some(cell) = buf.cell_mut(x, y)
                {
                    cell.set_symbol(rounded);
                    if dominant.fg != crate::style::Color::Reset {
                        cell.set_fg(dominant.fg);
                    }
                    continue;
                }
            }
            let table = if dominant.style == BorderStyle::Double {
                DOUBLE_TABLE
            } else {
                SOLID_TABLE
            };
            let replacement = table[mask as usize];
            if replacement.is_empty() {
                continue;
            }
            if let Some(cell) = buf.cell_mut(x, y) {
                cell.set_symbol(replacement);
                if dominant.fg != crate::style::Color::Reset {
                    cell.set_fg(dominant.fg);
                }
            }
        }
    }
}

/// True iff every visible direction at this cell has the SAME
/// priority. Used to detect "single-element corner" cells where
/// the rounded-corner fast path applies.
fn is_lone_contributor(cell_state: &[BorderDirState; 4], priority: u64) -> bool {
    for dir_state in cell_state {
        if !dir_state.is_visible() {
            continue;
        }
        let Some(c) = dir_state.winner else { continue };
        if c.priority != priority {
            return false;
        }
    }
    true
}

/// 4-bit visible-direction mask. Bit 0 = N, bit 1 = E, bit 2 = S,
/// bit 3 = W. A direction is "visible" iff its state has a
/// winning contribution AND was not killed by a Hidden
/// participant.
fn visible_mask(cell_state: &[BorderDirState; 4]) -> u8 {
    let mut mask = 0u8;
    if cell_state[DIR_N].is_visible() {
        mask |= 0b0001;
    }
    if cell_state[DIR_E].is_visible() {
        mask |= 0b0010;
    }
    if cell_state[DIR_S].is_visible() {
        mask |= 0b0100;
    }
    if cell_state[DIR_W].is_visible() {
        mask |= 0b1000;
    }
    mask
}

/// Pick the dominant contribution across the cell's visible
/// directions. CSS Tables 3 §11.5 says the higher-rank style wins
/// (`double > solid > dashed > …`); on a tie, the higher-priority
/// (more nested, then earlier-DOM) contribution wins. The dominant
/// contribution's style chooses the glyph table; its color paints
/// the cell.
fn dominant_contribution(cell_state: &[BorderDirState; 4]) -> BorderContribution {
    let mut best: Option<BorderContribution> = None;
    for dir_state in cell_state {
        if !dir_state.is_visible() {
            continue;
        }
        let Some(c) = dir_state.winner else { continue };
        let win = match best {
            None => true,
            Some(prev) => (c.style.rank(), c.priority) > (prev.style.rank(), prev.priority),
        };
        if win {
            best = Some(c);
        }
    }
    // The mask gate above (>= 1 visible direction) guarantees at
    // least one winner exists when we reach here. The fallback
    // (only triggered if invariants break) is `Solid` + default
    // color — the safe rendering choice.
    best.unwrap_or(BorderContribution {
        style: BorderStyle::Solid,
        fg: crate::style::Color::Reset,
        priority: 0,
        corner_style: CornerStyle::Square,
        side: BorderSide::Top,
    })
}

/// Half-block glyph for an inward-quadrant set (see
/// `Buffer::half_block_quads`). Index bits: `QUAD_TL=1, QUAD_TR=2,
/// QUAD_BL=4, QUAD_BR=8`. Every one of the 16 quadrant combinations has a
/// Unicode block element, so half-block borders can express *any* junction
/// (edges, corners, T-junctions, crosses, welds, full block) — unlike the
/// single-line tables, there are no gaps. `0` (no quadrants) returns "".
const HALF_BLOCK_QUAD_TABLE: [&str; 16] = [
    " ", // 0000 none (unused — caller skips quads == 0)
    "▘", // 0001 TL
    "▝", // 0010 TR
    "▀", // 0011 TL+TR — upper half (bottom edge)
    "▖", // 0100 BL
    "▌", // 0101 TL+BL — left half (right edge)
    "▞", // 0110 TR+BL — anti-diagonal
    "▛", // 0111 TL+TR+BL
    "▗", // 1000 BR
    "▚", // 1001 TL+BR — diagonal
    "▐", // 1010 TR+BR — right half (left edge)
    "▜", // 1011 TL+TR+BR
    "▄", // 1100 BL+BR — lower half (top edge)
    "▙", // 1101 TL+BL+BR
    "▟", // 1110 TR+BL+BR
    "█", // 1111 all — full block (welded stack)
];

/// Glyph for a half-block inward-quadrant set, or "" when empty.
fn half_block_quad_glyph(quads: u8) -> &'static str {
    if quads == 0 {
        return "";
    }
    HALF_BLOCK_QUAD_TABLE[(quads & 0b1111) as usize]
}

// ─── Glyph lookup tables ────────────────────────────────────────

/// Single-line junctions. Index encoding: bit0 = N, bit1 = E,
/// bit2 = S, bit3 = W. Empty string means "leave the cell alone".
const SOLID_TABLE: [&str; 16] = [
    "",  // 0000 - none
    "│", // 0001 - N only
    "─", // 0010 - E only
    "└", // 0011 - N+E
    "│", // 0100 - S only
    "│", // 0101 - N+S
    "┌", // 0110 - E+S
    "├", // 0111 - N+E+S
    "─", // 1000 - W only
    "┘", // 1001 - N+W
    "─", // 1010 - E+W
    "┴", // 1011 - N+E+W
    "┐", // 1100 - S+W
    "┤", // 1101 - N+S+W
    "┬", // 1110 - E+S+W
    "┼", // 1111 - all four
];

/// Double-line junctions. Same mask encoding as
/// [`SOLID_TABLE`]. Used when the cell's dominant style is
/// `BorderStyle::Double`.
const DOUBLE_TABLE: [&str; 16] = [
    "",  // 0000 - none
    "║", // 0001 - N only
    "═", // 0010 - E only
    "╚", // 0011 - N+E
    "║", // 0100 - S only
    "║", // 0101 - N+S
    "╔", // 0110 - E+S
    "╠", // 0111 - N+E+S
    "═", // 1000 - W only
    "╝", // 1001 - N+W
    "═", // 1010 - E+W
    "╩", // 1011 - N+E+W
    "╗", // 1100 - S+W
    "╣", // 1101 - N+S+W
    "╦", // 1110 - E+S+W
    "╬", // 1111 - all four
];

/// Rounded-corner glyphs for the lone-contributor fast path. Only
/// the four pure-corner masks have a rounded form; everything else
/// (edges, T-junctions, crosses) returns empty so the caller
/// falls back to the square table. Unicode has no rounded
/// T-junctions, so any overlap demotes to square automatically.
const ROUNDED_TABLE: [&str; 16] = [
    "",  // 0000
    "",  // 0001 N only — pure vertical, no rounded form
    "",  // 0010 E only — pure horizontal
    "╰", // 0011 N+E — bottom-left corner
    "",  // 0100 S only
    "",  // 0101 N+S
    "╭", // 0110 E+S — top-left corner
    "",  // 0111 N+E+S T-junction — square only
    "",  // 1000 W only
    "╯", // 1001 N+W — bottom-right corner
    "",  // 1010 E+W
    "",  // 1011 N+E+W
    "╮", // 1100 S+W — top-right corner
    "",  // 1101 N+S+W
    "",  // 1110 E+S+W
    "",  // 1111 all four
];
