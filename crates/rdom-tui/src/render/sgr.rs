//! SGR (Select Graphic Rendition) byte emission.
//!
//! Given a previous style and a new style, emit the minimal ANSI
//! escape sequence that transforms the terminal's current SGR state
//! into the new one. This is the heart of "don't re-emit what hasn't
//! changed" optimization.
//!
//! ## Encoding
//!
//! - Foreground: `\x1b[38;5;Nm` (indexed), `\x1b[38;2;R;G;Bm`
//!   (truecolor), `\x1b[39m` (reset). rdom is truecolor-only — every
//!   CSS named color expands to a 24-bit `Rgb` triple at parse time;
//!   no ANSI-16 short codes are emitted.
//! - Background: same with `48;5`, `48;2`, `49`.
//! - Modifier bits: `1` bold, `3` italic, `4` underline,
//!   `5` slow blink, `6` rapid blink, `7` reversed, `8` hidden,
//!   `9` crossed-out. Turn-off codes: `22` bold, `23` italic,
//!   `24` underline, `25` blink, `27` reversed, `28` hidden,
//!   `29` crossed-out.
//!
//! ## Style cache across frames
//!
//! `CrosstermBackend` keeps an `SgrState` across `draw()` calls. When
//! frame N ends, we leave the terminal with whatever SGR state the
//! last cell had; frame N+1 starts from that state and only emits
//! diffs. This saves ~30% of bytes in steady-state rendering.

use std::io::{self, Write};

use super::{Color, Modifier};

/// The SGR state we need to track across cells: fg, bg, and the
/// modifier bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgrState {
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}

impl SgrState {
    pub const RESET: SgrState = SgrState {
        fg: Color::Reset,
        bg: Color::Reset,
        modifier: Modifier::empty(),
    };
}

/// Emit the minimal SGR sequence transitioning `w` from `prev` to `new`.
/// Returns the new state (equal to `new`; returned for chaining).
pub fn emit_sgr_transition<W: Write>(
    w: &mut W,
    prev: SgrState,
    new: SgrState,
) -> io::Result<SgrState> {
    if prev == new {
        return Ok(new);
    }

    // Modifier diff.
    let mod_to_remove = prev.modifier.difference(new.modifier);
    let mod_to_add = new.modifier.difference(prev.modifier);

    for code in modifier_off_codes(mod_to_remove) {
        write_sgr(w, code)?;
    }
    for code in modifier_on_codes(mod_to_add) {
        write_sgr(w, code)?;
    }

    // fg diff.
    if prev.fg != new.fg {
        emit_fg(w, new.fg)?;
    }
    // bg diff.
    if prev.bg != new.bg {
        emit_bg(w, new.bg)?;
    }

    Ok(new)
}

/// Emit a sequence that fully resets SGR state to defaults
/// (`\x1b[0m`). Used by `Terminal::clear` and on clean shutdown.
pub fn emit_reset<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(b"\x1b[0m")
}

fn write_sgr<W: Write>(w: &mut W, code: u16) -> io::Result<()> {
    write!(w, "\x1b[{}m", code)
}

// ─── Modifier encoding ──────────────────────────────────────────────

fn modifier_on_codes(bits: Modifier) -> impl Iterator<Item = u16> {
    let mut out = Vec::new();
    if bits.contains(Modifier::BOLD) {
        out.push(1);
    }
    if bits.contains(Modifier::ITALIC) {
        out.push(3);
    }
    if bits.contains(Modifier::UNDERLINED) {
        out.push(4);
    }
    if bits.contains(Modifier::SLOW_BLINK) {
        out.push(5);
    }
    if bits.contains(Modifier::RAPID_BLINK) {
        out.push(6);
    }
    if bits.contains(Modifier::HIDDEN) {
        out.push(8);
    }
    if bits.contains(Modifier::CROSSED_OUT) {
        out.push(9);
    }
    out.into_iter()
}

fn modifier_off_codes(bits: Modifier) -> impl Iterator<Item = u16> {
    let mut out = Vec::new();
    if bits.contains(Modifier::BOLD) {
        out.push(22);
    }
    if bits.contains(Modifier::ITALIC) {
        out.push(23);
    }
    if bits.contains(Modifier::UNDERLINED) {
        out.push(24);
    }
    if bits.intersects(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK) {
        out.push(25);
    }
    if bits.contains(Modifier::HIDDEN) {
        out.push(28);
    }
    if bits.contains(Modifier::CROSSED_OUT) {
        out.push(29);
    }
    out.into_iter()
}

// ─── Color encoding ─────────────────────────────────────────────────

/// Emit an SGR sequence setting the foreground color. Truecolor-
/// only: `Rgb` emits `\x1b[38;2;r;g;b m`, `Indexed` emits
/// `\x1b[38;5;n m`, `Reset` emits `\x1b[39m`. No ANSI-16
/// quantization — every Rgb goes out as `38;2;r;g;b`.
fn emit_fg<W: Write>(w: &mut W, color: Color) -> io::Result<()> {
    match color {
        Color::Reset => write!(w, "\x1b[39m"),
        Color::Indexed(n) => write!(w, "\x1b[38;5;{}m", n),
        Color::Rgb(r, g, b) => write!(w, "\x1b[38;2;{};{};{}m", r, g, b),
    }
}

/// Mirror of [`emit_fg`] for background colors.
fn emit_bg<W: Write>(w: &mut W, color: Color) -> io::Result<()> {
    match color {
        Color::Reset => write!(w, "\x1b[49m"),
        Color::Indexed(n) => write!(w, "\x1b[48;5;{}m", n),
        Color::Rgb(r, g, b) => write!(w, "\x1b[48;2;{};{};{}m", r, g, b),
    }
}

// ─── Cursor positioning (CUP) ───────────────────────────────────────

/// Emit `\x1b[y;xH` — Cursor Position (1-indexed). Writer must have
/// already tracked that it doesn't know where the cursor is (or that
/// it's not at the target).
pub fn emit_cup<W: Write>(w: &mut W, x: u16, y: u16) -> io::Result<()> {
    write!(w, "\x1b[{};{}H", y.saturating_add(1), x.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_diff(prev: SgrState, new: SgrState) -> Vec<u8> {
        let mut buf = Vec::new();
        emit_sgr_transition(&mut buf, prev, new).unwrap();
        buf
    }

    // ── fg colors ────────────────────────────────────────────────────

    #[test]
    fn fg_rgb_pure_red_is_truecolor() {
        // Truecolor-only: every Rgb goes out as `38;2;r;g;b`,
        // regardless of whether it happens to match a legacy ANSI
        // named-color RGB.
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                fg: Color::Rgb(255, 0, 0),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[38;2;255;0;0m");
    }

    #[test]
    fn fg_rgb_lightcoral_is_truecolor() {
        // CSS `lightcoral` — plain truecolor, no ANSI-16 shortcut.
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                fg: Color::Rgb(240, 128, 128),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[38;2;240;128;128m");
    }

    #[test]
    fn fg_indexed() {
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                fg: Color::Indexed(204),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[38;5;204m");
    }

    #[test]
    fn fg_rgb_truecolor() {
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                fg: Color::Rgb(61, 144, 206),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[38;2;61;144;206m");
    }

    #[test]
    fn fg_reset() {
        let buf = emit_diff(
            SgrState {
                fg: Color::Rgb(255, 0, 0),
                ..SgrState::default()
            },
            SgrState::default(),
        );
        assert_eq!(buf, b"\x1b[39m");
    }

    // ── bg colors ────────────────────────────────────────────────────

    #[test]
    fn bg_rgb_pure_blue_is_truecolor() {
        // Same as fg_rgb_pure_red_is_truecolor for the bg slot.
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                bg: Color::Rgb(0, 0, 255),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[48;2;0;0;255m");
    }

    #[test]
    fn bg_rgb_truecolor() {
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                bg: Color::Rgb(10, 20, 30),
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[48;2;10;20;30m");
    }

    // ── modifiers ────────────────────────────────────────────────────

    #[test]
    fn bold_on() {
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                modifier: Modifier::BOLD,
                ..SgrState::default()
            },
        );
        assert_eq!(buf, b"\x1b[1m");
    }

    #[test]
    fn multiple_modifiers() {
        let buf = emit_diff(
            SgrState::default(),
            SgrState {
                modifier: Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED,
                ..SgrState::default()
            },
        );
        // Codes emitted in order: 1, 3, 4
        assert_eq!(buf, b"\x1b[1m\x1b[3m\x1b[4m");
    }

    #[test]
    fn bold_off_uses_sgr22() {
        let buf = emit_diff(
            SgrState {
                modifier: Modifier::BOLD,
                ..SgrState::default()
            },
            SgrState::default(),
        );
        assert_eq!(buf, b"\x1b[22m");
    }

    #[test]
    fn italic_off_uses_sgr23() {
        let buf = emit_diff(
            SgrState {
                modifier: Modifier::ITALIC,
                ..SgrState::default()
            },
            SgrState::default(),
        );
        assert_eq!(buf, b"\x1b[23m");
    }

    #[test]
    fn underline_off_uses_sgr24() {
        let buf = emit_diff(
            SgrState {
                modifier: Modifier::UNDERLINED,
                ..SgrState::default()
            },
            SgrState::default(),
        );
        assert_eq!(buf, b"\x1b[24m");
    }

    // ── no-op transitions ────────────────────────────────────────────

    #[test]
    fn same_state_emits_nothing() {
        let state = SgrState {
            fg: Color::Rgb(255, 0, 0),
            bg: Color::Rgb(0, 0, 0),
            modifier: Modifier::BOLD,
        };
        assert_eq!(emit_diff(state, state), b"");
    }

    #[test]
    fn only_fg_changed() {
        let prev = SgrState {
            fg: Color::Rgb(255, 0, 0),
            bg: Color::Rgb(0, 0, 0),
            modifier: Modifier::BOLD,
        };
        let new = SgrState {
            fg: Color::Rgb(0, 128, 0),
            ..prev
        };
        assert_eq!(emit_diff(prev, new), b"\x1b[38;2;0;128;0m");
    }

    // ── combined transitions ─────────────────────────────────────────

    #[test]
    fn add_bold_change_fg_and_bg() {
        let prev = SgrState::default();
        let new = SgrState {
            fg: Color::Rgb(255, 0, 0),
            bg: Color::Rgb(0, 0, 0),
            modifier: Modifier::BOLD,
        };
        let buf = emit_diff(prev, new);
        // Modifiers first (add bold), then fg, then bg — all truecolor.
        assert_eq!(buf, b"\x1b[1m\x1b[38;2;255;0;0m\x1b[48;2;0;0;0m");
    }

    #[test]
    fn remove_one_bit_add_another() {
        let prev = SgrState {
            modifier: Modifier::BOLD,
            ..SgrState::default()
        };
        let new = SgrState {
            modifier: Modifier::ITALIC,
            ..SgrState::default()
        };
        // Remove bold (22), add italic (3).
        assert_eq!(emit_diff(prev, new), b"\x1b[22m\x1b[3m");
    }

    // ── CUP (cursor position) ───────────────────────────────────────

    #[test]
    fn cup_emits_one_indexed_row_col() {
        let mut buf = Vec::new();
        emit_cup(&mut buf, 5, 10).unwrap();
        assert_eq!(buf, b"\x1b[11;6H");
    }

    #[test]
    fn cup_origin_is_1_1() {
        let mut buf = Vec::new();
        emit_cup(&mut buf, 0, 0).unwrap();
        assert_eq!(buf, b"\x1b[1;1H");
    }

    // ── Reset ───────────────────────────────────────────────────────

    #[test]
    fn emit_reset_produces_sgr0() {
        let mut buf = Vec::new();
        emit_reset(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[0m");
    }
}
