//! `Color` — terminal color model.
//!
//! Four shapes:
//!
//! - **`Reset`** — the terminal's default foreground or background.
//!   Emits SGR `39` / `49` (reset fg / reset bg) — the one color that
//!   doesn't set a specific value, just releases the slot.
//! - **`Indexed(u8)`** — the 256-color palette (`\x1b[38;5;Nm`); the
//!   terminal applies its own palette (the first 16 entries are its
//!   theme's ANSI colors).
//! - **`Rgb(r, g, b)`** — truecolor (`\x1b[38;2;R;G;Bm`), full 24-bit:
//!   what every CSS color keyword and color function computes to.
//! - **`Rgba(r, g, b, a)`** — truecolor with alpha below 255 (CSS
//!   Color 4 §4.2: `rgb(255 0 0 / 50%)`, `#ff000080`, `transparent`).
//!   A terminal cell is opaque, so paint composites a translucent
//!   color over what lies beneath before it reaches a cell.
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

mod absolute;
mod convert;
mod gamut;
mod interpolate;
mod matrices;
pub mod named;
pub mod palette;
mod scheme;
pub(crate) mod system;

pub use scheme::{ColorScheme, ColorSchemeList};
pub use system::SystemColor;

pub use interpolate::interpolate_oklab;
pub(crate) use interpolate::{HueMethod, mix};

pub(crate) use absolute::{AbsoluteColor, ColorSpace};
pub(crate) use convert::convert;

/// Terminal color. Four variants: `Reset` (terminal default),
/// `Indexed` (xterm-256 palette index), `Rgb` (24-bit truecolor) and
/// `Rgba` (truecolor with alpha).
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

    /// 24-bit truecolor with an alpha below 255 (CSS Color 4 §4.2):
    /// `0` is fully transparent. An opaque color is always `Rgb` —
    /// build one with [`Color::rgba`], which normalizes, so equal
    /// colors compare equal.
    Rgba(u8, u8, u8, u8),
}

impl Color {
    /// `transparent` (CSS Color 4 §6.3): transparent black.
    pub const TRANSPARENT: Color = Color::Rgba(0, 0, 0, 0);

    /// A truecolor with alpha `a` (`255` opaque): `Rgb` when opaque,
    /// `Rgba` otherwise.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        if a == u8::MAX {
            Color::Rgb(r, g, b)
        } else {
            Color::Rgba(r, g, b, a)
        }
    }

    /// The alpha channel, `0` (transparent) to `255` (opaque). Every
    /// color but `Rgba` is opaque.
    pub const fn alpha(self) -> u8 {
        match self {
            Color::Rgba(_, _, _, a) => a,
            _ => u8::MAX,
        }
    }

    /// True when the alpha channel is below 255.
    pub const fn is_translucent(self) -> bool {
        self.alpha() < u8::MAX
    }

    /// This color with its alpha dropped (`Rgba` becomes `Rgb`).
    pub const fn opaque(self) -> Color {
        match self {
            Color::Rgba(r, g, b, _) => Color::Rgb(r, g, b),
            c => c,
        }
    }

    /// True when this color is `Reset` — helpful for paint paths that
    /// want to avoid emitting a full SGR when the effective color is
    /// "whatever the terminal default is."
    pub fn is_reset(self) -> bool {
        matches!(self, Color::Reset)
    }
}

/// `color`'s Oklch coordinates `[L, C, h]` (tests of the gamut mapping).
#[cfg(test)]
pub(crate) fn oklch_of(color: Color) -> [f64; 3] {
    let abs = AbsoluteColor::from_color(color).expect("an sRGB color");
    convert::convert(abs, ColorSpace::Oklch).values()
}

/// An 8-bit alpha as the CSSOM serializes it (CSS Color 4 §15.2): the
/// shortest of two or three decimals that reads back as the same byte
/// (`128` is `0.5`, `1` is `0.004`).
pub fn serialize_alpha(a: u8) -> String {
    let byte = |v: f64| (v * 255.0 + 0.5).floor() as u8;
    let v = f64::from(a) / 255.0;
    let two = (v * 100.0).round() / 100.0;
    let v = if byte(two) == a {
        two
    } else {
        (v * 1000.0).round() / 1000.0
    };
    format!("{v}")
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

    /// CSS Color 4 §12.1: colors interpolate in Oklab, premultiplied —
    /// red to blue passes through a light purple, not sRGB's dark one,
    /// and white to black through Oklab's perceptual mid gray.
    #[test]
    fn interpolation_is_in_oklab() {
        let mid = interpolate_oklab(Color::Rgb(255, 0, 0), Color::Rgb(0, 0, 255), 0.5);
        assert_eq!(mid, Some(Color::Rgb(140, 83, 162)));
        let gray = interpolate_oklab(Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0), 0.5);
        assert_eq!(gray, Some(Color::Rgb(99, 99, 99)));
        let fade = interpolate_oklab(Color::TRANSPARENT, Color::Rgb(255, 0, 0), 0.5);
        assert_eq!(fade, Some(Color::Rgba(255, 0, 0, 128)));
        assert_eq!(
            interpolate_oklab(Color::Reset, Color::Rgb(1, 2, 3), 0.5),
            None
        );
    }

    /// CSS Color 4 §15.2: alpha serializes with the fewest decimals
    /// that round-trip.
    #[test]
    fn alpha_serializes_shortest_round_trip() {
        assert_eq!(serialize_alpha(128), "0.5");
        assert_eq!(serialize_alpha(0), "0");
        assert_eq!(serialize_alpha(64), "0.25");
        assert_eq!(serialize_alpha(1), "0.004");
    }

    /// CSS Color 4 §4.2: an opaque color has one representation, so
    /// `rgba(…, 255)` and `Rgb` compare equal.
    #[test]
    fn rgba_normalizes_opaque_alpha() {
        assert_eq!(Color::rgba(1, 2, 3, 255), Color::Rgb(1, 2, 3));
        assert_eq!(Color::rgba(1, 2, 3, 128), Color::Rgba(1, 2, 3, 128));
        assert_eq!(Color::Rgba(1, 2, 3, 128).alpha(), 128);
        assert_eq!(Color::Rgb(1, 2, 3).alpha(), 255);
        assert_eq!(Color::Reset.alpha(), 255);
        assert!(Color::TRANSPARENT.is_translucent());
        assert_eq!(Color::Rgba(1, 2, 3, 9).opaque(), Color::Rgb(1, 2, 3));
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
