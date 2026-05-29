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
            // Half-block style — direction-asymmetric glyphs picked
            // from (mask, side) per the lone-element half-block
            // table. T-junctions / shared cells don't have a
            // well-defined glyph; fall through and skip them.
            //
            // The half-block branch deliberately leaves the cell's
            // bg ALONE. The paint flow upstream (see
            // `paint_pass::mod`'s `fill_bg` call site, which clips
            // to padding-box when `border_has_half_block`) skipped
            // painting the element's own bg under these border
            // cells — so the cell still carries whatever the
            // parent painted earlier. Result: each half-block
            // glyph's "empty" half visually merges into the parent
            // bg, producing the pill silhouette. This is rdom's
            // hard-coded analog of CSS `background-clip:
            // padding-box`; see DIVERGENCES.md.
            if dominant.style == BorderStyle::HalfBlock && lone {
                let glyph = half_block_glyph(&cell_state, mask);
                if !glyph.is_empty()
                    && let Some(cell) = buf.cell_mut(x, y)
                {
                    cell.set_symbol(glyph);
                    if dominant.fg != crate::style::Color::Reset {
                        cell.set_fg(dominant.fg);
                    }
                }
                continue;
            }
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

/// Half-block glyph lookup for `BorderStyle::HalfBlock`. The
/// direction mask alone can't distinguish a top edge from a bottom
/// edge (both have E+W set), so the joiner also reads the side tag
/// carried on each `BorderContribution` to pick the right
/// asymmetric glyph.
///
/// Glyph mapping:
/// - `▄` U+2584 LOWER HALF BLOCK — top edge cell
/// - `▀` U+2580 UPPER HALF BLOCK — bottom edge cell
/// - `▐` U+2590 RIGHT HALF BLOCK — left edge cell (color on right half)
/// - `▌` U+258C LEFT HALF BLOCK  — right edge cell (color on left half)
/// - `▗` U+2597 QUADRANT LOWER RIGHT — top-left corner
/// - `▖` U+2596 QUADRANT LOWER LEFT  — top-right corner
/// - `▝` U+259D QUADRANT UPPER RIGHT — bottom-left corner
/// - `▘` U+2598 QUADRANT UPPER LEFT  — bottom-right corner
///
/// Each glyph paints its color on the half/quarter that points
/// INWARD toward the bordered element's content — so the colored
/// regions of adjacent border cells join up into a continuous pill
/// shape.
///
/// T-junctions and shared cells (multiple contributors) return ""
/// — `HalfBlock` is designed for the lone-element case and the
/// joiner gates this branch on `is_lone_contributor`.
fn half_block_glyph(cell_state: &[BorderDirState; 4], mask: u8) -> &'static str {
    let side = |dir: usize| -> Option<BorderSide> { cell_state[dir].winner.map(|c| c.side) };
    match mask {
        // E+W — top or bottom edge (run continues left and right).
        0b1010 => match side(DIR_E).or_else(|| side(DIR_W)) {
            Some(BorderSide::Top) => "▄",
            Some(BorderSide::Bottom) => "▀",
            _ => "",
        },
        // N+S — left or right edge (run continues up and down).
        0b0101 => match side(DIR_N).or_else(|| side(DIR_S)) {
            Some(BorderSide::Left) => "▐",
            Some(BorderSide::Right) => "▌",
            _ => "",
        },
        // E+S — top-left corner.
        0b0110 => "▗",
        // S+W — top-right corner.
        0b1100 => "▖",
        // N+E — bottom-left corner.
        0b0011 => "▝",
        // N+W — bottom-right corner.
        0b1001 => "▘",
        // Single-bit cells, T-junctions, all-four: no half-block
        // glyph — HalfBlock is for the lone-element ring case.
        _ => "",
    }
}
