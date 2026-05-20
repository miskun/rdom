//! `Color` — terminal color model.
//!
//! Four flavours:
//!
//! - **`Reset`** — the terminal's default foreground or background.
//!   Emits SGR `39` / `49` (reset fg / reset bg) — the one color that
//!   doesn't set a specific value, just releases the slot.
//! - **ANSI-16 named** (`Black`, `Red`, ..., `White`, plus the eight
//!   `Light*` variants) — the classic `0..=15` palette. Most themeable,
//!   least precise; two different terminals will render them
//!   differently. *(Marked for removal: subsequent OOTB pre-publish
//!   commits delete the ANSI variants in favor of CSS named colors —
//!   see `color::named`. Browser-faithful naming wins over the ANSI
//!   16-entry theme-dependent palette.)*
//! - **`Indexed(u8)`** — 256-color palette (`\x1b[38;5;Nm`). Wider
//!   gamut than ANSI-16 but still discrete; the lower 16 entries
//!   overlap with the ANSI-16 set.
//! - **`Rgb(r, g, b)`** — truecolor (`\x1b[38;2;R;G;Bm`). Full 24-bit.
//!   The future-canonical wire format; once the ANSI variants are
//!   gone, `Rgb` and `Indexed` are the only non-`Reset` shapes.
//!
//! `Color` values are `Copy` and cheap — no heap allocation, no
//! indirection. The SGR serialization lives in `render/sgr.rs`.
//!
//! ## CSS named colors
//!
//! The 148 keywords from CSS Color Module Level 4 §6.1 are
//! available as `pub const` items in [`named`]:
//!
//! ```
//! use rdom_style::{Color, color::named};
//! assert_eq!(named::DODGERBLUE, Color::Rgb(30, 144, 255));
//! ```
//!
//! Plus runtime case-insensitive lookup via [`named::lookup`]
//! (used by the CSS parser when it sees `color: rebeccapurple`).

pub mod named;

/// Terminal color. Three variants: `Reset` (terminal default),
/// `Indexed` (xterm-256 palette index), `Rgb` (24-bit truecolor).
/// The 16 ANSI named variants (Black/Red/.../White) were removed
/// in the pre-publish OOTB color overhaul (T6) — rdom is
/// truecolor-only, and the CSS named colors in [`named`] cover
/// what the ANSI variants used to.
///
/// `Default` = `Color::Reset` so types that embed a `Color` get a
/// sensible default without a manual impl everywhere.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// Terminal default (SGR 39 / 49). The cascade's initial value
    /// for `fg`/`bg`.
    #[default]
    Reset,

    /// xterm-256 palette index. Authors who specifically want
    /// "xterm color 208" write `Color::Indexed(208)` for that
    /// intent. Emitted as `\x1b[38;5;n m` so the terminal applies
    /// its own palette mapping — the escape hatch for "I want the
    /// 256-color palette, not a literal RGB triple." `Rgb` is the
    /// canonical, theme-independent form and what every CSS color
    /// keyword in [`named`] expands to.
    Indexed(u8),

    /// 24-bit truecolor. Authors construct directly
    /// (`Color::Rgb(30, 144, 255)`) or via the CSS named
    /// constants in [`named`] (`named::DODGERBLUE`).
    Rgb(u8, u8, u8),
}

impl Color {
    /// True when this color is `Reset` — helpful for paint paths that
    /// want to avoid emitting a full SGR when the effective color is
    /// "whatever the terminal default is."
    pub fn is_reset(self) -> bool {
        matches!(self, Color::Reset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_predicate() {
        assert!(Color::Reset.is_reset());
        assert!(!Color::Rgb(255, 0, 0).is_reset());
        assert!(!Color::Rgb(0, 0, 0).is_reset());
        assert!(!Color::Indexed(200).is_reset());
    }

    #[test]
    fn color_is_copy_and_eq() {
        let a = Color::Rgb(255, 0, 0);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn indexed_full_range() {
        // Make sure u8 covers the entire 256-color palette.
        for i in 0u8..=255 {
            let c = Color::Indexed(i);
            match c {
                Color::Indexed(n) => assert_eq!(n, i),
                _ => unreachable!(),
            }
        }
    }
}
