//! Border join pass — single source of truth for border glyphs +
//! colors. Runs after the main paint pass.
//!
//! `paint_border` records per-cell × per-direction contributions in
//! `buf.border_dirs`. Each contribution carries a [`BorderStyle`],
//! a foreground color, and a structural priority. Inside each
//! direction the contributions are reconciled per CSS Tables 3
//! §11.5: `hidden` kills the direction; otherwise the heaviest,
//! then highest-rank, then highest-priority contribution wins.
//!
//! The joiner walks the buffer once and, for every cell that has
//! at least one visible direction, emits the junction glyph + the
//! winning direction's foreground color. The glyph is chosen
//! (`glyphs::junction_glyph`) by the line each direction carries:
//!
//! - `BorderStyle::Double` → a double line (`║═╔╗╚╝╠╣╦╩╬`).
//! - `Dashed` / `Dotted` on a straight run (N + S or E + W, one weight)
//!   → Unicode's dash glyphs (`╌╎` / `┄┆`, heavy `╍╏` / `┅┇`); their
//!   corners and junctions have no dashed form and take the single-line
//!   glyph below.
//! - Anything else (`Solid`, `Ridge`, `Outset`, `Groove`, `Inset`) → a
//!   single line, light (`│─┌┐└┘├┤┬┴┼`) or heavy (`┃━┏┓┗┛┣┫┳┻╋`) by its
//!   winner's `border-width`, the mixed junctions (`┍┿╽`) where weights
//!   meet. The 3-D keywords parse and rank correctly in conflict
//!   resolution but degrade to the single-line glyph set on the
//!   terminal — CSS-faithful "render as best you can" per the medium
//!   constraint documented in `DIVERGENCES.md`.
//! - A double axis crossing a light one → Unicode's mixed glyphs
//!   (`╒╓╕╖╘╙╛╜╞╟╡╢╤╥╧╨╪╫`). Unicode has no glyph where a heavy line
//!   meets a double one, nor where one axis is double on one side and
//!   single on the other: there the cell's dominant style picks the
//!   table (double, or single by weight).
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

mod glyphs;

use glyphs::{
    DOUBLE_TABLE, Line, ROUNDED_TABLE, dash_glyph, half_block_quad_glyph, junction_glyph,
    line_glyph,
};
use rdom_core::Dom;
use rdom_style::layout::{BorderStyle, BorderWeight, CornerStyle};

use crate::ext::TuiExt;
use crate::render::Buffer;
use crate::render::buffer::{
    BorderContribution, BorderDirState, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W,
};

/// The glyph of an outline ring cell (`outline`, CSS UI 4 §5) that joins
/// the ring cells beside it in `joins` (N, E, S, W), drawn in `line` at
/// `weight` — a rounded corner where `rounded` (`outline-style: auto`) —
/// from the border glyph tables; `None` for a cell that joins nothing.
pub(super) fn outline_glyph(
    joins: [bool; 4],
    line: BorderStyle,
    weight: BorderWeight,
    rounded: bool,
) -> Option<&'static str> {
    let kind = match (line, weight) {
        (BorderStyle::Double, _) => Line::Double,
        (_, BorderWeight::Heavy) => Line::Heavy,
        (_, BorderWeight::Light) => Line::Light,
    };
    let lines = joins.map(|j| if j { kind } else { Line::None });
    let mask = joins
        .iter()
        .enumerate()
        .filter(|(_, j)| **j)
        .fold(0usize, |m, (i, _)| m | 1 << i);
    if mask == 0 {
        return None;
    }
    if rounded && kind == Line::Light && !ROUNDED_TABLE[mask].is_empty() {
        return Some(ROUNDED_TABLE[mask]);
    }
    dash_glyph(lines, line)
        .or_else(|| junction_glyph(lines))
        .filter(|g| !g.is_empty())
}

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
                        draw_border_glyph(cell, glyph, fg);
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
            let lines = line_kinds(&cell_state);
            // Unicode's rounded corners are light single lines only.
            if lone
                && dominant.corner_style == CornerStyle::Rounded
                && lines.iter().all(|l| matches!(l, Line::None | Line::Light))
            {
                let rounded = ROUNDED_TABLE[mask as usize];
                if !rounded.is_empty()
                    && let Some(cell) = buf.cell_mut(x, y)
                {
                    draw_border_glyph(cell, rounded, dominant.fg);
                    continue;
                }
            }
            // A straight run of a dashed / dotted line draws Unicode's
            // dash glyphs; its corners and junctions stay solid.
            let replacement = dash_glyph(lines, dominant.style)
                .or_else(|| junction_glyph(lines))
                .unwrap_or_else(|| {
                    if dominant.style == BorderStyle::Double {
                        DOUBLE_TABLE[mask as usize]
                    } else {
                        line_glyph(line_weights(&cell_state))
                    }
                });
            if replacement.is_empty() {
                continue;
            }
            if let Some(cell) = buf.cell_mut(x, y) {
                draw_border_glyph(cell, replacement, dominant.fg);
            }
        }
    }
}

/// Draw a border glyph in its border's color — the default (`Reset`,
/// `currentcolor` of a default-coloured box) included — with nothing of
/// the glyph it replaces (ACID-FIX-3); the cell's background stays.
fn draw_border_glyph(cell: &mut crate::render::Cell, glyph: &str, fg: crate::style::Color) {
    cell.set_symbol(glyph);
    cell.clear_glyph_style();
    cell.set_fg(fg);
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

/// Each direction's line weight, N, E, S, W: `0` not visible, `1`
/// light, `2` heavy (its winner's `border-width`).
fn line_weights(cell_state: &[BorderDirState; 4]) -> [u8; 4] {
    cell_state.map(|d| match d.winner {
        Some(c) if d.is_visible() => match c.weight {
            BorderWeight::Light => 1,
            BorderWeight::Heavy => 2,
        },
        _ => 0,
    })
}

/// The line each direction carries, N, E, S, W: none, a double line
/// (its winner's style is `double`), or a light / heavy single line.
fn line_kinds(cell_state: &[BorderDirState; 4]) -> [Line; 4] {
    cell_state.map(|d| match d.winner {
        Some(c) if d.is_visible() => match (c.style, c.weight) {
            (BorderStyle::Double, _) => Line::Double,
            (_, BorderWeight::Light) => Line::Light,
            (_, BorderWeight::Heavy) => Line::Heavy,
        },
        _ => Line::None,
    })
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
///
/// Within one element — a corner, where two of its sides meet — the
/// browser splits the cell between the sides, the wider one taking
/// more (CSS Backgrounds 3 §4.4 "corner shaping" geometry); a cell has
/// one color, so the dominant side takes it whole: the wider (heavier
/// weight), then the heavier style, and on a tie the horizontal side
/// (top / bottom), so the top and bottom of a box read as whole lines
/// (DIVERGENCES §2). Across elements, §11.5 likewise ranks width first.
fn dominant_contribution(cell_state: &[BorderDirState; 4]) -> BorderContribution {
    let dominance = |c: &BorderContribution| {
        let horizontal = matches!(c.side, BorderSide::Top | BorderSide::Bottom);
        (c.weight, c.style.rank(), c.priority, horizontal)
    };
    let mut best: Option<BorderContribution> = None;
    for dir_state in cell_state {
        if !dir_state.is_visible() {
            continue;
        }
        let Some(c) = dir_state.winner else { continue };
        let win = match best {
            None => true,
            Some(prev) => dominance(&c) > dominance(&prev),
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
        weight: BorderWeight::Light,
        priority: 0,
        corner_style: CornerStyle::Square,
        side: BorderSide::Top,
    })
}
