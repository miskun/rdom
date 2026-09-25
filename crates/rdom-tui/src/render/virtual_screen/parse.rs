//! ANSI byte-stream parsing for [`VirtualScreen`]: CSI dispatch
//! (cursor movement, clear, cursor visibility, synchronized output),
//! SGR decoding into the screen's pen state, and UTF-8 grapheme
//! splitting of literal text.

use unicode_segmentation::UnicodeSegmentation;

use super::VirtualScreen;
use crate::render::sgr::SgrState;
use crate::render::{Cell, Color, Modifier};

impl VirtualScreen {
    /// Parse and apply an ANSI byte stream. Multiple calls accumulate.
    pub fn apply(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0x1b {
                // Escape sequence.
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'[' => {
                        i += 1;
                        i += self.parse_csi(&bytes[i..]);
                    }
                    _ => {
                        // Unknown / single-char escape — skip this byte.
                        i += 1;
                    }
                }
            } else {
                // Literal byte — could be start of a multi-byte UTF-8
                // grapheme. Find the grapheme that starts here.
                let (g, consumed) = next_grapheme(&bytes[i..]);
                self.write_grapheme(g);
                i += consumed;
            }
        }
    }

    /// Parse and apply a CSI sequence (after `\x1b[`). Returns the
    /// number of bytes consumed from the `after` slice.
    fn parse_csi(&mut self, after: &[u8]) -> usize {
        // A CSI sequence is: (params) (intermediate bytes) (final byte).
        // Params: digits, `;`, `?`.
        // Final byte: a letter (0x40–0x7E).
        let mut end = 0;
        while end < after.len() {
            let c = after[end];
            if c.is_ascii_alphabetic() || c == b'@' || c == b'`' || c == b'~' {
                break;
            }
            end += 1;
        }
        if end >= after.len() {
            return after.len();
        }
        let final_byte = after[end];
        let params_str = std::str::from_utf8(&after[..end]).unwrap_or("");
        let total_consumed = end + 1;

        match final_byte {
            b'H' | b'f' => {
                // CUP — \x1b[y;xH (1-indexed; missing = 1).
                let (row, col) = parse_pair(params_str);
                let y = row
                    .saturating_sub(1)
                    .min(self.height.saturating_sub(1) as u32) as u16;
                let x = col
                    .saturating_sub(1)
                    .min(self.width.saturating_sub(1) as u32) as u16;
                self.cursor = (x, y);
            }
            b'J' => {
                // Erase in Display. \x1b[2J = all; 0J = to end; 1J = to start.
                // For our backend only 2J is used.
                let mode: u32 = params_str.parse().unwrap_or(0);
                if mode == 2 {
                    for c in &mut self.cells {
                        *c = Cell::EMPTY;
                    }
                }
            }
            b'm' => {
                // SGR.
                self.apply_sgr(params_str);
            }
            b'h' | b'l' => {
                // Private mode set (`h`) / reset (`l`). \x1b[?25l etc.
                let set = final_byte == b'h';
                let trimmed = params_str.trim_start_matches('?');
                for part in trimmed.split(';') {
                    match part {
                        "25" => self.cursor_visible = set,
                        "2026" => { /* BSU/ESU — no grid effect */ }
                        _ => { /* ignore other modes */ }
                    }
                }
            }
            _ => {
                // Unknown CSI — skip silently.
            }
        }
        total_consumed
    }

    /// Apply an SGR parameter string like "0", "31", "38;5;204", "1;31;40".
    fn apply_sgr(&mut self, params: &str) {
        // Walk tokens; some codes consume following tokens.
        let tokens: Vec<&str> = params.split(';').collect();
        let mut i = 0;
        if tokens.is_empty() || (tokens.len() == 1 && tokens[0].is_empty()) {
            // Bare \x1b[m means reset.
            self.sgr = SgrState::default();
            return;
        }
        while i < tokens.len() {
            let n: u32 = tokens[i].parse().unwrap_or(0);
            match n {
                0 => self.sgr = SgrState::default(),
                1 => self.sgr.modifier |= Modifier::BOLD,
                // SGR-2 (dim) was mapped to `Modifier::DIM` pre-T8;
                // that bit is gone (no CSS analog, terminal-dependent).
                // Incoming SGR-2 is now silently ignored.
                2 => { /* dim — ignored, see T8 */ }
                3 => self.sgr.modifier |= Modifier::ITALIC,
                4 => self.sgr.modifier |= Modifier::UNDERLINED,
                5 => self.sgr.modifier |= Modifier::SLOW_BLINK,
                6 => self.sgr.modifier |= Modifier::RAPID_BLINK,
                // SGR-7 (reverse video) was emitted by rdom pre-
                // caret-color rollout. Now ignored — rdom paints
                // explicit fg/bg for the caret cell instead.
                7 => { /* reverse video — no internal flag */ }
                8 => self.sgr.modifier |= Modifier::HIDDEN,
                9 => self.sgr.modifier |= Modifier::CROSSED_OUT,
                22 => self.sgr.modifier.remove(Modifier::BOLD),
                23 => self.sgr.modifier.remove(Modifier::ITALIC),
                24 => self.sgr.modifier.remove(Modifier::UNDERLINED),
                25 => self
                    .sgr
                    .modifier
                    .remove(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK),
                27 => { /* reverse-video off — no internal flag */ }
                28 => self.sgr.modifier.remove(Modifier::HIDDEN),
                29 => self.sgr.modifier.remove(Modifier::CROSSED_OUT),
                30..=37 => self.sgr.fg = ansi16_color((n - 30) as u8, false),
                38 => {
                    // Extended fg. "38;5;N" or "38;2;R;G;B".
                    if let Some(mode) = tokens.get(i + 1) {
                        match mode.parse::<u32>().unwrap_or(0) {
                            5 => {
                                if let Some(idx) = tokens.get(i + 2) {
                                    let n: u8 = idx.parse().unwrap_or(0);
                                    self.sgr.fg = Color::Indexed(n);
                                    i += 2;
                                }
                            }
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (tokens.get(i + 2), tokens.get(i + 3), tokens.get(i + 4))
                                {
                                    let r = r.parse().unwrap_or(0);
                                    let g = g.parse().unwrap_or(0);
                                    let b = b.parse().unwrap_or(0);
                                    self.sgr.fg = Color::Rgb(r, g, b);
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.sgr.fg = Color::Reset,
                40..=47 => self.sgr.bg = ansi16_color((n - 40) as u8, false),
                48 => {
                    if let Some(mode) = tokens.get(i + 1) {
                        match mode.parse::<u32>().unwrap_or(0) {
                            5 => {
                                if let Some(idx) = tokens.get(i + 2) {
                                    let n: u8 = idx.parse().unwrap_or(0);
                                    self.sgr.bg = Color::Indexed(n);
                                    i += 2;
                                }
                            }
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (tokens.get(i + 2), tokens.get(i + 3), tokens.get(i + 4))
                                {
                                    let r = r.parse().unwrap_or(0);
                                    let g = g.parse().unwrap_or(0);
                                    let b = b.parse().unwrap_or(0);
                                    self.sgr.bg = Color::Rgb(r, g, b);
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.sgr.bg = Color::Reset,
                90..=97 => self.sgr.fg = ansi16_color((n - 90) as u8, true),
                100..=107 => self.sgr.bg = ansi16_color((n - 100) as u8, true),
                _ => { /* unknown SGR — ignore */ }
            }
            i += 1;
        }
    }
}

// ─── helpers ────────────────────────────────────────────────────────

fn parse_pair(s: &str) -> (u32, u32) {
    let mut parts = s.split(';');
    let a = parts.next().and_then(|t| t.parse().ok()).unwrap_or(1);
    let b = parts.next().and_then(|t| t.parse().ok()).unwrap_or(1);
    (a, b)
}

/// Decode an ANSI-16 SGR code (`\x1b[30-37m` base, `\x1b[90-97m`
/// bright) back into a 24-bit `Color::Rgb`. rdom itself never emits
/// these short codes since T7 (we're truecolor-only on the wire),
/// but `VirtualScreen` still has to *parse* arbitrary incoming SGR
/// streams (replay, test harnesses, anything that wasn't produced
/// by rdom). The triples below are the canonical xterm RGB values
/// for the 16 ANSI palette slots — close to the legacy variants
/// that lived in `Color` pre-T6.
fn ansi16_color(code: u8, bright: bool) -> Color {
    if bright {
        match code {
            0 => Color::Rgb(169, 169, 169),
            1 => Color::Rgb(240, 128, 128),
            2 => Color::Rgb(144, 238, 144),
            3 => Color::Rgb(255, 255, 224),
            4 => Color::Rgb(173, 216, 230),
            5 => Color::Rgb(255, 128, 255),
            6 => Color::Rgb(224, 255, 255),
            7 => Color::Rgb(255, 255, 255),
            _ => Color::Reset,
        }
    } else {
        match code {
            0 => Color::Rgb(0, 0, 0),
            1 => Color::Rgb(255, 0, 0),
            2 => Color::Rgb(0, 128, 0),
            3 => Color::Rgb(255, 255, 0),
            4 => Color::Rgb(0, 0, 255),
            5 => Color::Rgb(255, 0, 255),
            6 => Color::Rgb(0, 255, 255),
            7 => Color::Rgb(128, 128, 128),
            _ => Color::Reset,
        }
    }
}

/// Return the next grapheme cluster starting at the beginning of
/// `bytes`, plus the byte length consumed. Bytes past the first
/// grapheme are left untouched.
fn next_grapheme(bytes: &[u8]) -> (&str, usize) {
    // Find a valid UTF-8 prefix.
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            // Partial UTF-8 at end. Try the valid prefix.
            let valid_up_to = e.valid_up_to();
            if valid_up_to == 0 {
                // First byte is invalid — return a single-byte fallback.
                return ("?", 1);
            }
            std::str::from_utf8(&bytes[..valid_up_to]).unwrap()
        }
    };
    let mut graphemes = s.graphemes(true);
    match graphemes.next() {
        Some(g) => (g, g.len()),
        None => ("", 0),
    }
}
