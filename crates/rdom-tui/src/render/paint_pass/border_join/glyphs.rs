//! The border glyph tables the joiner picks from: single-line
//! junctions by per-direction weight (light / heavy), double-line
//! junctions, the double / single mixes, the rounded corners, the
//! dashed / dotted runs, and the half-block quadrant set.

use rdom_style::layout::BorderStyle;

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

/// The line one direction of a junction carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Line {
    None,
    Light,
    Heavy,
    Double,
}

/// The glyph where `lines` (N, E, S, W) meet, or `None` where Unicode
/// has none: a heavy line meeting a double one, or a double and a
/// single line on one axis (`╒` exists, a double-above-single vertical
/// does not). Single lines pick by weight ([`line_glyph`]); all-double
/// junctions from [`DOUBLE_TABLE`]; a double axis crossing a light one
/// from the mixed tables (`╒╓╕╖╘╙╛╜╞╟╡╢╤╥╧╨╪╫`, U+2552–U+256B).
pub(super) fn junction_glyph(lines: [Line; 4]) -> Option<&'static str> {
    let weight = |l: Line| match l {
        Line::None => 0,
        Line::Light => 1,
        Line::Heavy | Line::Double => 2,
    };
    if !lines.contains(&Line::Double) {
        return Some(line_glyph(lines.map(weight)));
    }
    if lines.contains(&Line::Heavy) {
        return None;
    }
    let mask = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| **l != Line::None)
        .fold(0usize, |m, (i, _)| m | 1 << i);
    let [n, e, s, w] = lines;
    // The kind one axis carries: `None` when it is empty or mixed.
    let axis = |a: Line, b: Line| match (a, b) {
        (Line::None, x) | (x, Line::None) => Some(x),
        (x, y) if x == y => Some(x),
        _ => None,
    };
    match (axis(n, s)?, axis(e, w)?) {
        (Line::Double | Line::None, Line::Double | Line::None) => Some(DOUBLE_TABLE[mask]),
        (Line::Double, _) => Some(VERTICAL_DOUBLE_TABLE[mask]),
        _ => Some(HORIZONTAL_DOUBLE_TABLE[mask]),
    }
}

/// The dash glyph of a straight run of a `dashed` or `dotted` line (CSS
/// Backgrounds 3 §4.2: a series of dashes / dots), or `None` for any
/// other style or cell. `lines` must be one straight axis — N + S or
/// E + W, or one end of one where a side stops, one weight: Unicode has no dashed corner, junction or weight
/// mix, so those cells keep the solid glyph. `dashed` is the double dash
/// (`╌╎`, heavy `╍╏`), `dotted` the triple dash (`┄┆`, heavy `┅┇`) —
/// the finer pattern; each glyph checked against its Unicode name
/// ("… DOUBLE DASH …", "… TRIPLE DASH …").
pub(super) fn dash_glyph(lines: [Line; 4], style: BorderStyle) -> Option<&'static str> {
    // [style][vertical?][heavy?]
    const DASHES: [[[&str; 2]; 2]; 2] = [[["╌", "╍"], ["╎", "╏"]], [["┄", "┅"], ["┆", "┇"]]];
    let pattern = match style {
        BorderStyle::Dashed => 0,
        BorderStyle::Dotted => 1,
        _ => return None,
    };
    // A straight run: both ends of one axis, or one end where the side
    // stops with no other side meeting it (a box with no bottom border:
    // its left side's last cell) — no direction changes in either.
    let (vertical, line) = match lines {
        [n, Line::None, s, Line::None] if n == s => (1, n),
        [Line::None, e, Line::None, w] if e == w => (0, e),
        [l, Line::None, Line::None, Line::None] | [Line::None, Line::None, l, Line::None] => (1, l),
        [Line::None, l, Line::None, Line::None] | [Line::None, Line::None, Line::None, l] => (0, l),
        _ => return None,
    };
    let heavy = match line {
        Line::Light => 0,
        Line::Heavy => 1,
        Line::None | Line::Double => return None,
    };
    Some(DASHES[pattern][vertical][heavy])
}

/// A double vertical line (N, S) meeting light horizontal ones (E, W),
/// indexed like [`DOUBLE_TABLE`]; each glyph checked against its
/// Unicode name ("… DOUBLE AND … SINGLE").
const VERTICAL_DOUBLE_TABLE: [&str; 16] = [
    "",  // 0000
    "║", // 0001 N
    "─", // 0010 E
    "╙", // 0011 N+E — UP DOUBLE AND RIGHT SINGLE
    "║", // 0100 S
    "║", // 0101 N+S
    "╓", // 0110 E+S — DOWN DOUBLE AND RIGHT SINGLE
    "╟", // 0111 N+E+S — VERTICAL DOUBLE AND RIGHT SINGLE
    "─", // 1000 W
    "╜", // 1001 N+W — UP DOUBLE AND LEFT SINGLE
    "─", // 1010 E+W
    "╨", // 1011 N+E+W — UP DOUBLE AND HORIZONTAL SINGLE
    "╖", // 1100 S+W — DOWN DOUBLE AND LEFT SINGLE
    "╢", // 1101 N+S+W — VERTICAL DOUBLE AND LEFT SINGLE
    "╥", // 1110 E+S+W — DOWN DOUBLE AND HORIZONTAL SINGLE
    "╫", // 1111 — VERTICAL DOUBLE AND HORIZONTAL SINGLE
];

/// A double horizontal line (E, W) meeting light vertical ones (N, S),
/// indexed like [`DOUBLE_TABLE`] ("… SINGLE AND … DOUBLE").
const HORIZONTAL_DOUBLE_TABLE: [&str; 16] = [
    "",  // 0000
    "│", // 0001 N
    "═", // 0010 E
    "╘", // 0011 N+E — UP SINGLE AND RIGHT DOUBLE
    "│", // 0100 S
    "│", // 0101 N+S
    "╒", // 0110 E+S — DOWN SINGLE AND RIGHT DOUBLE
    "╞", // 0111 N+E+S — VERTICAL SINGLE AND RIGHT DOUBLE
    "═", // 1000 W
    "╛", // 1001 N+W — UP SINGLE AND LEFT DOUBLE
    "═", // 1010 E+W
    "╧", // 1011 N+E+W — UP SINGLE AND HORIZONTAL DOUBLE
    "╕", // 1100 S+W — DOWN SINGLE AND LEFT DOUBLE
    "╡", // 1101 N+S+W — VERTICAL SINGLE AND LEFT DOUBLE
    "╤", // 1110 E+S+W — DOWN SINGLE AND HORIZONTAL DOUBLE
    "╪", // 1111 — VERTICAL SINGLE AND HORIZONTAL DOUBLE
];

/// Double-line junctions. Index encoding: bit0 = N, bit1 = E, bit2 =
/// S, bit3 = W. Used when every line at the cell is double, or — where
/// no mixed glyph exists — when the cell's dominant style is
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

#[cfg(test)]
mod tests {
    use super::Line::{Double as D, Heavy as H, Light as L, None as O};
    use super::*;

    /// `C4G-MIXED-CORNERS`: every mixed corner, T and cross, by the
    /// lines it joins (N, E, S, W); the gaps fall back to the caller.
    #[test]
    fn double_and_single_lines_pick_the_mixed_glyphs() {
        for (lines, glyph) in [
            ([O, D, L, O], "╒"),
            ([O, L, D, O], "╓"),
            ([O, O, L, D], "╕"),
            ([O, O, D, L], "╖"),
            ([L, D, O, O], "╘"),
            ([D, L, O, O], "╙"),
            ([L, O, O, D], "╛"),
            ([D, O, O, L], "╜"),
            ([L, D, L, O], "╞"),
            ([D, L, D, O], "╟"),
            ([L, O, L, D], "╡"),
            ([D, O, D, L], "╢"),
            ([O, D, L, D], "╤"),
            ([O, L, D, L], "╥"),
            ([L, D, O, D], "╧"),
            ([D, L, O, L], "╨"),
            ([L, D, L, D], "╪"),
            ([D, L, D, L], "╫"),
            ([O, D, D, O], "╔"),
            ([D, D, D, D], "╬"),
            ([O, L, H, O], "┎"),
        ] {
            assert_eq!(junction_glyph(lines), Some(glyph), "{lines:?}");
        }
        // No glyph: heavy meets double; double and single on one axis.
        assert_eq!(junction_glyph([O, H, D, O]), None);
        assert_eq!(junction_glyph([D, L, L, O]), None);
        assert_eq!(junction_glyph([L, D, O, L]), None);
    }

    /// `C4G-EDGE-TESTS`: the dash glyphs by code point (U+2504–U+2507
    /// TRIPLE DASH, U+254C–U+254F DOUBLE DASH), straight runs and a run's
    /// lone end only.
    #[test]
    fn dashed_and_dotted_runs_pick_the_dash_glyphs() {
        use BorderStyle::{Dashed, Dotted, Solid};
        for (lines, style, code) in [
            ([O, L, O, L], Dashed, 0x254C),
            ([O, H, O, H], Dashed, 0x254D),
            ([L, O, L, O], Dashed, 0x254E),
            ([H, O, H, O], Dashed, 0x254F),
            ([O, L, O, L], Dotted, 0x2504),
            ([O, H, O, H], Dotted, 0x2505),
            ([L, O, L, O], Dotted, 0x2506),
            ([H, O, H, O], Dotted, 0x2507),
            // A side's end where no other side meets it is a straight
            // run too, not a corner (ACID-FIX-2).
            ([L, O, O, O], Dashed, 0x254E),
            ([O, O, O, H], Dotted, 0x2505),
        ] {
            let glyph = dash_glyph(lines, style).unwrap();
            assert_eq!(glyph.chars().next().map(u32::from), Some(code), "{lines:?}");
        }
        // Corners, junctions, mixed weights, other styles.
        for (lines, style) in [
            ([O, L, L, O], Dashed),
            ([L, L, L, L], Dotted),
            ([L, O, H, O], Dashed),
            ([O, D, O, D], Dashed),
            ([O, L, O, L], Solid),
        ] {
            assert_eq!(dash_glyph(lines, style), None, "{lines:?} {style:?}");
        }
    }
}
