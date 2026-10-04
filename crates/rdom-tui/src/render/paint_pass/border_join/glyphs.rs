//! The border glyph tables the joiner picks from: single-line
//! junctions by per-direction weight (light / heavy), double-line
//! junctions, the rounded corners, and the half-block quadrant set.

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
pub(super) fn half_block_quad_glyph(quads: u8) -> &'static str {
    if quads == 0 {
        return "";
    }
    HALF_BLOCK_QUAD_TABLE[(quads & 0b1111) as usize]
}

// ─── Glyph lookup tables ────────────────────────────────────────

/// Single-line junctions, light and heavy (U+2500–U+254B): the glyph
/// for each direction's weight — none, light (`─│`, `thin` / `medium`
/// borders) or heavy (`━┃`, `thick`) — indexed by [`line_index`].
/// Every one of the 81 combinations has a glyph, the mixed ones (`┍`,
/// `┿`, `╽`, …) included; a lone direction draws the full line of its
/// weight. Generated from the Unicode character names.
const LINE_TABLE: [&str; 81] = [
    "",  // N0 E0 S0 W0
    "│", // N1 E0 S0 W0
    "┃", // N2 E0 S0 W0
    "─", // N0 E1 S0 W0
    "└", // N1 E1 S0 W0
    "┖", // N2 E1 S0 W0
    "━", // N0 E2 S0 W0
    "┕", // N1 E2 S0 W0
    "┗", // N2 E2 S0 W0
    "│", // N0 E0 S1 W0
    "│", // N1 E0 S1 W0
    "╿", // N2 E0 S1 W0
    "┌", // N0 E1 S1 W0
    "├", // N1 E1 S1 W0
    "┞", // N2 E1 S1 W0
    "┍", // N0 E2 S1 W0
    "┝", // N1 E2 S1 W0
    "┡", // N2 E2 S1 W0
    "┃", // N0 E0 S2 W0
    "╽", // N1 E0 S2 W0
    "┃", // N2 E0 S2 W0
    "┎", // N0 E1 S2 W0
    "┟", // N1 E1 S2 W0
    "┠", // N2 E1 S2 W0
    "┏", // N0 E2 S2 W0
    "┢", // N1 E2 S2 W0
    "┣", // N2 E2 S2 W0
    "─", // N0 E0 S0 W1
    "┘", // N1 E0 S0 W1
    "┚", // N2 E0 S0 W1
    "─", // N0 E1 S0 W1
    "┴", // N1 E1 S0 W1
    "┸", // N2 E1 S0 W1
    "╼", // N0 E2 S0 W1
    "┶", // N1 E2 S0 W1
    "┺", // N2 E2 S0 W1
    "┐", // N0 E0 S1 W1
    "┤", // N1 E0 S1 W1
    "┦", // N2 E0 S1 W1
    "┬", // N0 E1 S1 W1
    "┼", // N1 E1 S1 W1
    "╀", // N2 E1 S1 W1
    "┮", // N0 E2 S1 W1
    "┾", // N1 E2 S1 W1
    "╄", // N2 E2 S1 W1
    "┒", // N0 E0 S2 W1
    "┧", // N1 E0 S2 W1
    "┨", // N2 E0 S2 W1
    "┰", // N0 E1 S2 W1
    "╁", // N1 E1 S2 W1
    "╂", // N2 E1 S2 W1
    "┲", // N0 E2 S2 W1
    "╆", // N1 E2 S2 W1
    "╊", // N2 E2 S2 W1
    "━", // N0 E0 S0 W2
    "┙", // N1 E0 S0 W2
    "┛", // N2 E0 S0 W2
    "╾", // N0 E1 S0 W2
    "┵", // N1 E1 S0 W2
    "┹", // N2 E1 S0 W2
    "━", // N0 E2 S0 W2
    "┷", // N1 E2 S0 W2
    "┻", // N2 E2 S0 W2
    "┑", // N0 E0 S1 W2
    "┥", // N1 E0 S1 W2
    "┩", // N2 E0 S1 W2
    "┭", // N0 E1 S1 W2
    "┽", // N1 E1 S1 W2
    "╃", // N2 E1 S1 W2
    "┯", // N0 E2 S1 W2
    "┿", // N1 E2 S1 W2
    "╇", // N2 E2 S1 W2
    "┓", // N0 E0 S2 W2
    "┪", // N1 E0 S2 W2
    "┫", // N2 E0 S2 W2
    "┱", // N0 E1 S2 W2
    "╅", // N1 E1 S2 W2
    "╉", // N2 E1 S2 W2
    "┳", // N0 E2 S2 W2
    "╈", // N1 E2 S2 W2
    "╋", // N2 E2 S2 W2
];

/// [`LINE_TABLE`]'s index for per-direction weights (`0` none, `1`
/// light, `2` heavy) in N, E, S, W order.
fn line_index(weights: [u8; 4]) -> usize {
    let [n, e, s, w] = weights.map(usize::from);
    n + 3 * e + 9 * s + 27 * w
}

/// The single-line glyph for per-direction weights (N, E, S, W; `0`
/// none, `1` light, `2` heavy), or "" when every direction is none.
pub(super) fn line_glyph(weights: [u8; 4]) -> &'static str {
    LINE_TABLE[line_index(weights)]
}

/// Double-line junctions. Index encoding: bit0 = N, bit1 = E, bit2 =
/// S, bit3 = W. Used when the cell's dominant style is
/// `BorderStyle::Double`.
pub(super) const DOUBLE_TABLE: [&str; 16] = [
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
pub(super) const ROUNDED_TABLE: [&str; 16] = [
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
