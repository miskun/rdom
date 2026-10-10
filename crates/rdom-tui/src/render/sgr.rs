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
//!   (truecolor), `\x1b[39m` (reset). Styles hold 24-bit colors; each
//!   is emitted at the terminal's [`ColorDepth`] — its nearest palette
//!   index at 256 colors, an ANSI-16 code (`3n` / `9n`) at 16, nothing
//!   but the default with no color (C16G-COLOR-DEPTH).
//! - Background: same with `48;5`, `48;2`, `49`.
//! - Modifier bits: `1` bold, `3` italic, `4` underline,
//!   `5` slow blink, `6` rapid blink, `7` reversed, `8` hidden,
//!   `9` crossed-out. Turn-off codes: `22` bold, `23` italic,
//!   `24` underline, `25` blink, `27` reversed, `28` hidden,
//!   `29` crossed-out.
//! - The decorations beyond ECMA-48's common subset, only to a terminal
//!   whose [`SgrCapabilities`] say it understands them: the underline
//!   styles `4:1`–`4:5` (colon sub-parameters, which an older terminal
//!   would misread as separate codes — without them every style is a
//!   plain `4`), the underline color `58:2::r:g:b` / `58:5:n` / `59`,
//!   and the overline `53` / `55`.
//!
//! ## Style cache across frames
//!
//! `CrosstermBackend` keeps an `SgrState` across `draw()` calls. When
//! frame N ends, we leave the terminal with whatever SGR state the
//! last cell had; frame N+1 starts from that state and only emits
//! diffs. This saves ~30% of bytes in steady-state rendering.

use std::io::{self, Write};

pub use super::sgr_capabilities::SgrCapabilities;
use super::{Color, Modifier};
use crate::ColorDepth;

/// The SGR state we need to track across cells: fg, bg, the modifier
/// bitmask and the underline color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgrState {
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    /// The underline color (SGR 58); `Color::Reset`, the default (SGR 59).
    pub underline_color: Color,
}

impl SgrState {
    pub const RESET: SgrState = SgrState {
        fg: Color::Reset,
        bg: Color::Reset,
        modifier: Modifier::empty(),
        underline_color: Color::Reset,
    };
}

/// Emit the minimal SGR sequence transitioning `w` from `prev` to `new`
/// for a terminal of [`SgrCapabilities::BASIC`].
/// Returns the new state (equal to `new`; returned for chaining).
pub fn emit_sgr_transition<W: Write>(
    w: &mut W,
    prev: SgrState,
    new: SgrState,
) -> io::Result<SgrState> {
    emit_sgr_transition_for(w, prev, new, SgrCapabilities::BASIC)
}

/// Emit the minimal SGR sequence transitioning `w` from `prev` to `new`
/// for a terminal that understands `caps`: a decoration it does not is
/// left out (an underline style degrades to a plain underline).
/// Returns the new state (equal to `new`; returned for chaining).
pub fn emit_sgr_transition_for<W: Write>(
    w: &mut W,
    prev: SgrState,
    new: SgrState,
    caps: SgrCapabilities,
) -> io::Result<SgrState> {
    emit_sgr_transition_at(w, prev, new, caps, ColorDepth::TrueColor)
}

/// [`emit_sgr_transition_for`] for a terminal showing `depth`: each
/// color is emitted as the nearest the terminal shows
/// ([`ColorDepth::quantize`]) — at 16 colors as SGR 30–37 / 90–97 (and
/// their backgrounds), which a 16-color terminal reads — and the state
/// returned (and compared next) is the quantized one, so two colors
/// with the same nearest one emit nothing between them.
pub fn emit_sgr_transition_at<W: Write>(
    w: &mut W,
    prev: SgrState,
    new: SgrState,
    caps: SgrCapabilities,
    depth: ColorDepth,
) -> io::Result<SgrState> {
    let new = SgrState {
        fg: depth.quantize(new.fg),
        bg: depth.quantize(new.bg),
        underline_color: depth.quantize(new.underline_color),
        ..new
    };
    let ansi16 = depth == ColorDepth::Ansi16;
    if prev == new {
        return Ok(new);
    }

    // Modifier diff — the underline and the overline apart.
    let mod_to_remove = prev.modifier.difference(new.modifier);
    let mod_to_add = new.modifier.difference(prev.modifier);

    // In code order: the underline (`4` / `24`) between italic and blink.
    let (head, tail) = MODIFIER_CODES.split_at(2);
    let (was, is) = (
        underline_of(prev.modifier, caps),
        underline_of(new.modifier, caps),
    );
    for code in modifier_off_codes(head, mod_to_remove) {
        write_sgr(w, code)?;
    }
    if was.is_some() && is.is_none() {
        write_sgr(w, 24)?;
    }
    for code in modifier_off_codes(tail, mod_to_remove) {
        write_sgr(w, code)?;
    }
    if caps.overline && mod_to_remove.contains(Modifier::OVERLINED) {
        write_sgr(w, 55)?;
    }
    for code in modifier_on_codes(head, mod_to_add) {
        write_sgr(w, code)?;
    }
    match is {
        Some(style) if was != is => match style {
            0 => write_sgr(w, 4)?,
            n => write!(w, "\x1b[4:{n}m")?,
        },
        _ => {}
    }
    for code in modifier_on_codes(tail, mod_to_add) {
        write_sgr(w, code)?;
    }
    if caps.overline && mod_to_add.contains(Modifier::OVERLINED) {
        write_sgr(w, 53)?;
    }

    // fg diff.
    if prev.fg != new.fg {
        emit_fg(w, new.fg, ansi16)?;
    }
    // bg diff.
    if prev.bg != new.bg {
        emit_bg(w, new.bg, ansi16)?;
    }
    // Underline color diff.
    if caps.underline_color && prev.underline_color != new.underline_color {
        emit_underline_color(w, new.underline_color)?;
    }

    Ok(new)
}

/// The underline `modifier` asks a terminal of `caps` for: `None` for
/// none, `Some(0)` for a plain one (SGR 4), `Some(n)` for the style `n`
/// of SGR `4:n` (2 double, 3 curly, 4 dotted, 5 dashed) — only where the
/// terminal takes colon sub-parameters.
fn underline_of(modifier: Modifier, caps: SgrCapabilities) -> Option<u8> {
    if !modifier.contains(Modifier::UNDERLINED) {
        return None;
    }
    if !caps.styled_underline {
        return Some(0);
    }
    const STYLES: [(Modifier, u8); 4] = [
        (Modifier::UNDERLINE_DOUBLE, 2),
        (Modifier::UNDERLINE_CURLY, 3),
        (Modifier::UNDERLINE_DOTTED, 4),
        (Modifier::UNDERLINE_DASHED, 5),
    ];
    Some(
        STYLES
            .iter()
            .find(|(bit, _)| modifier.contains(*bit))
            .map_or(0, |&(_, n)| n),
    )
}

/// Emit SGR 58 (the underline color, kitty's colon form, the color space
/// id empty) or 59 (the default).
fn emit_underline_color<W: Write>(w: &mut W, color: Color) -> io::Result<()> {
    match color {
        Color::Reset => write!(w, "\x1b[59m"),
        Color::Indexed(n) => write!(w, "\x1b[58:5:{n}m"),
        Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => write!(w, "\x1b[58:2::{r}:{g}:{b}m"),
    }
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

/// Modifier bits with their SGR turn-on and turn-off codes. Both blink
/// bits share the `25` turn-off code, emitted once (see
/// [`modifier_off_codes`]). The underline (with its styles) and the
/// overline depend on the terminal (`emit_sgr_transition_for`).
const MODIFIER_CODES: [(Modifier, u16, u16); 6] = [
    (Modifier::BOLD, 1, 22),
    (Modifier::ITALIC, 3, 23),
    (Modifier::SLOW_BLINK, 5, 25),
    (Modifier::RAPID_BLINK, 6, 25),
    (Modifier::HIDDEN, 8, 28),
    (Modifier::CROSSED_OUT, 9, 29),
];

/// Turn-on codes for the set bits, in table order. Allocation-free:
/// filters the const table.
fn modifier_on_codes(
    table: &'static [(Modifier, u16, u16)],
    bits: Modifier,
) -> impl Iterator<Item = u16> {
    table
        .iter()
        .filter(move |(bit, _, _)| bits.contains(*bit))
        .map(|(_, on, _)| *on)
}

/// Turn-off codes for the set bits, in table order, each code once
/// (both blink bits map to `25`).
fn modifier_off_codes(
    table: &'static [(Modifier, u16, u16)],
    bits: Modifier,
) -> impl Iterator<Item = u16> {
    let mut last: Option<u16> = None;
    table
        .iter()
        .filter(move |(bit, _, _)| bits.contains(*bit))
        .filter_map(move |(_, _, off)| {
            if last == Some(*off) {
                None
            } else {
                last = Some(*off);
                Some(*off)
            }
        })
}

// ─── Color encoding ─────────────────────────────────────────────────

/// Emit an SGR sequence setting the foreground color: `Rgb` as
/// `\x1b[38;2;r;g;b m`, `Indexed` as `\x1b[38;5;n m` — or, for a
/// 16-color terminal (`ansi16`, the color already quantized to 0–15),
/// `\x1b[3n m` / `\x1b[9n m` — and `Reset` as `\x1b[39m`. The colors
/// arrive quantized to the terminal's depth ([`emit_sgr_transition_at`]).
fn emit_fg<W: Write>(w: &mut W, color: Color, ansi16: bool) -> io::Result<()> {
    match color {
        Color::Reset => write!(w, "\x1b[39m"),
        Color::Indexed(n) if ansi16 && n < 8 => write!(w, "\x1b[{}m", 30 + n),
        Color::Indexed(n) if ansi16 && n < 16 => write!(w, "\x1b[{}m", 90 + n - 8),
        Color::Indexed(n) => write!(w, "\x1b[38;5;{}m", n),
        Color::Rgb(r, g, b) => write!(w, "\x1b[38;2;{};{};{}m", r, g, b),
        // A cell is opaque: paint composites alpha away before a color
        // reaches one (`Buffer::write_styled`), so only a cell built
        // field by field holds one, and its channels are emitted.
        Color::Rgba(r, g, b, _) => write!(w, "\x1b[38;2;{};{};{}m", r, g, b),
    }
}

/// Mirror of [`emit_fg`] for background colors.
fn emit_bg<W: Write>(w: &mut W, color: Color, ansi16: bool) -> io::Result<()> {
    match color {
        Color::Reset => write!(w, "\x1b[49m"),
        Color::Indexed(n) if ansi16 && n < 8 => write!(w, "\x1b[{}m", 40 + n),
        Color::Indexed(n) if ansi16 && n < 16 => write!(w, "\x1b[{}m", 100 + n - 8),
        Color::Indexed(n) => write!(w, "\x1b[48;5;{}m", n),
        Color::Rgb(r, g, b) => write!(w, "\x1b[48;2;{};{};{}m", r, g, b),
        Color::Rgba(r, g, b, _) => write!(w, "\x1b[48;2;{};{};{}m", r, g, b),
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
            underline_color: Color::Reset,
        };
        assert_eq!(emit_diff(state, state), b"");
    }

    #[test]
    fn only_fg_changed() {
        let prev = SgrState {
            fg: Color::Rgb(255, 0, 0),
            bg: Color::Rgb(0, 0, 0),
            modifier: Modifier::BOLD,
            underline_color: Color::Reset,
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
            underline_color: Color::Reset,
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
