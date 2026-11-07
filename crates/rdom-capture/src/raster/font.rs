//! The bundled bitmap font (`assets/unscii-16.hex`, public domain — see
//! `assets/NOTICE.md`): 8×16 glyphs, a few 16×16, parsed once on first use.
//! Its box-drawing, block and Braille glyphs are drawn for the 8×16 cell
//! and meet their neighbours at its edges, so lines join exactly at every
//! integer scale.

use std::collections::HashMap;
use std::sync::OnceLock;

/// One glyph: its rows, top first, the leftmost pixel in the high bit of
/// a 16-bit row (an 8-wide glyph uses the high byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Glyph {
    /// 16 pixels wide (a wide character's two cells), else 8.
    pub wide: bool,
    pub rows: [u16; 16],
}

impl Glyph {
    /// Whether pixel `(x, y)` is set.
    pub fn at(&self, x: usize, y: usize) -> bool {
        y < 16 && x < if self.wide { 16 } else { 8 } && self.rows[y] & (0x8000 >> x) != 0
    }
}

const FONT: &str = include_str!("../../assets/unscii-16.hex");

fn glyphs() -> &'static HashMap<u32, Glyph> {
    static GLYPHS: OnceLock<HashMap<u32, Glyph>> = OnceLock::new();
    GLYPHS.get_or_init(|| FONT.lines().filter_map(parse).collect())
}

/// One `.hex` line: `codepoint:bitmap`.
fn parse(line: &str) -> Option<(u32, Glyph)> {
    let (cp, hex) = line.trim().split_once(':')?;
    let cp = u32::from_str_radix(cp, 16).ok()?;
    let wide = match hex.len() {
        32 => false,
        64 => true,
        _ => return None,
    };
    let per = if wide { 4 } else { 2 };
    let mut rows = [0u16; 16];
    for (y, row) in rows.iter_mut().enumerate() {
        let v = u16::from_str_radix(&hex[y * per..(y + 1) * per], 16).ok()?;
        *row = if wide { v } else { v << 8 };
    }
    Some((cp, Glyph { wide, rows }))
}

/// The glyph of character `c`, if the font has it.
pub(crate) fn glyph(c: char) -> Option<&'static Glyph> {
    glyphs().get(&u32::from(c))
}

#[cfg(test)]
mod tests {
    use super::glyph;

    #[test]
    fn the_font_covers_what_terminals_draw() {
        for c in "AZaz09~ äöÅ€ ─│┌┐└┘├┤┬┴┼═║╔╗╭╮ ▀▄█▌▐░▒▓ ⠿⡇ ΩЖ"
            .chars()
            .filter(|c| *c != ' ')
        {
            assert!(glyph(c).is_some(), "{c:?} ({:04X})", u32::from(c));
        }
        let a = glyph('A').unwrap();
        assert!(!a.wide);
        assert!((0..16).any(|y| (0..8).any(|x| a.at(x, y))), "A has ink");
        assert!(!a.at(8, 0), "8 wide");
        // A full block fills its cell.
        let full = glyph('█').unwrap();
        assert!((0..16).all(|y| (0..8).all(|x| full.at(x, y))));
        // A horizontal line reaches both edges, so neighbours join.
        let h = glyph('─').unwrap();
        assert!((0..16).any(|y| h.at(0, y) && h.at(7, y)));
    }
}
