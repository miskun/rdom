//! How many colors a terminal shows, and the nearest one it has
//! (C16G-COLOR-DEPTH): 24-bit color, the xterm 256-color palette, the 16
//! ANSI colors, or none (the `NO_COLOR` convention). The backend
//! quantizes each color as it emits it — styles keep their sRGB values —
//! and the media features `color`, `color-index` and `monochrome` (Media
//! Queries 4 §6.1–§6.3) read the depth.
//!
//! The nearest palette color is the nearest in Oklab (CSS Color 4 §9.2,
//! the perceptual space CSS gamut-maps in): for the 256-color palette,
//! among the 6 × 6 × 6 cube's eight entries around the color and the
//! gray ramp's nearest — not indices 0–15, which follow the user's theme
//! (`palette`); for 16 colors, among those sixteen at xterm's defaults.

use super::Color;
use super::palette::xterm_rgb;

/// How many colors the terminal shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ColorDepth {
    /// No color: every color is the terminal's default (`NO_COLOR`,
    /// <https://no-color.org>); bold, underline and the other attributes
    /// stay.
    NoColor,
    /// The 16 ANSI colors (SGR 30–37, 90–97 and their backgrounds).
    Ansi16,
    /// The xterm 256-color palette (SGR `38;5;n`).
    Ansi256,
    /// 24-bit color (SGR `38;2;r;g;b`).
    #[default]
    TrueColor,
}

impl ColorDepth {
    /// The depth the environment variables describe, read through `var`:
    /// `NO_COLOR` set and not empty → [`NoColor`](Self::NoColor) (its
    /// convention: any value but the empty string); `COLORTERM` of
    /// `truecolor` or `24bit` → [`TrueColor`](Self::TrueColor), as is a
    /// Windows Terminal session (`WT_SESSION`, which sets no `COLORTERM`
    /// and draws 24-bit color); a `TERM` ending in `-256color` →
    /// [`Ansi256`](Self::Ansi256); anything else
    /// [`Ansi16`](Self::Ansi16).
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Self {
        if var("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return ColorDepth::NoColor;
        }
        let colorterm = var("COLORTERM").unwrap_or_default();
        if matches!(colorterm.as_str(), "truecolor" | "24bit") || var("WT_SESSION").is_some() {
            return ColorDepth::TrueColor;
        }
        if var("TERM").is_some_and(|t| t.ends_with("-256color")) {
            return ColorDepth::Ansi256;
        }
        ColorDepth::Ansi16
    }

    /// Media Queries 4 §6.1 `color`: bits per color component — 8 for
    /// 24-bit color, 2 for the 256-color palette (8 bits a pixel), 1 for
    /// 16 colors (4 bits a pixel), 0 with no color.
    pub fn bits(self) -> u8 {
        match self {
            ColorDepth::TrueColor => 8,
            ColorDepth::Ansi256 => 2,
            ColorDepth::Ansi16 => 1,
            ColorDepth::NoColor => 0,
        }
    }

    /// Media Queries 4 §6.2 `color-index`: the entries of the color
    /// lookup table — 256 and 16 for the palettes, 0 for 24-bit color
    /// (no table) and with no color.
    pub fn index_entries(self) -> u32 {
        match self {
            ColorDepth::Ansi256 => 256,
            ColorDepth::Ansi16 => 16,
            ColorDepth::TrueColor | ColorDepth::NoColor => 0,
        }
    }

    /// `color` as the terminal shows it: unchanged at 24 bits (alpha
    /// dropped — a cell is opaque); an sRGB color as its nearest palette
    /// index, and a palette index past 15 as its nearest of the sixteen,
    /// at 16 colors; [`Color::Reset`] with no color.
    pub fn quantize(self, color: Color) -> Color {
        match (self, color) {
            (_, Color::Reset) => Color::Reset,
            (ColorDepth::NoColor, _) => Color::Reset,
            (ColorDepth::TrueColor, Color::Rgba(r, g, b, _)) => Color::Rgb(r, g, b),
            (ColorDepth::TrueColor, c) => c,
            (ColorDepth::Ansi256, Color::Indexed(n)) => Color::Indexed(n),
            (ColorDepth::Ansi256, Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _)) => {
                Color::Indexed(nearest_256((r, g, b)))
            }
            (ColorDepth::Ansi16, Color::Indexed(n)) if n < 16 => Color::Indexed(n),
            (ColorDepth::Ansi16, Color::Indexed(n)) => Color::Indexed(nearest_16(xterm_rgb(n))),
            (ColorDepth::Ansi16, Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _)) => {
                Color::Indexed(nearest_16((r, g, b)))
            }
        }
    }
}

/// The 256-color palette index nearest `rgb` in Oklab, outside 0–15: the
/// cube's entries at the two levels around each channel, and the gray
/// ramp's entry nearest its mean.
pub fn nearest_256(rgb: (u8, u8, u8)) -> u8 {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let around = |c: u8| {
        let hi = LEVELS.iter().position(|&l| l >= c).unwrap_or(5);
        [hi.saturating_sub(1), hi]
    };
    let (rs, gs, bs) = (around(rgb.0), around(rgb.1), around(rgb.2));
    let cube = rs.into_iter().flat_map(|r| {
        gs.into_iter()
            .flat_map(move |g| bs.into_iter().map(move |b| 16 + 36 * r + 6 * g + b))
    });
    let mean = (u16::from(rgb.0) + u16::from(rgb.1) + u16::from(rgb.2)) / 3;
    let step = (mean.saturating_sub(3) / 10).min(23) as usize;
    let grays = [step.saturating_sub(1), step, (step + 1).min(23)].map(|s| 232 + s);
    nearest(rgb, cube.chain(grays).map(|i| i as u8))
}

/// The ANSI color (0–15, at xterm's defaults) nearest `rgb` in Oklab.
pub fn nearest_16(rgb: (u8, u8, u8)) -> u8 {
    nearest(rgb, 0..16)
}

/// The candidate palette index nearest `rgb` in Oklab (the first of a
/// tie).
fn nearest(rgb: (u8, u8, u8), candidates: impl Iterator<Item = u8>) -> u8 {
    let target = oklab(rgb);
    let distance = |i: u8| {
        let c = oklab(xterm_rgb(i));
        (0..3).map(|k| (c[k] - target[k]).powi(2)).sum::<f64>()
    };
    candidates
        .map(|i| (distance(i), i))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(0, |(_, i)| i)
}

/// `rgb`'s Oklab coordinates (CSS Color 4 §9.2's matrices, through
/// linear sRGB).
fn oklab(rgb: (u8, u8, u8)) -> [f64; 3] {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(rgb.0), lin(rgb.1), lin(rgb.2));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

#[cfg(test)]
#[path = "depth_tests.rs"]
mod tests;
