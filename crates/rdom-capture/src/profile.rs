//! Terminal profiles (`specs/CAPTURE.md` §Terminal profiles): what a real
//! terminal makes of a frame that rdom's paint does not decide — the 16
//! ANSI colors and the default ones, whether bold also brightens, which
//! attributes it draws. A profile turns a
//! painted [`ColorValue`] into the sRGB a viewer sees, for the image views
//! and the contrast audits.

use serde::{Deserialize, Serialize};

use crate::capture::ColorValue;

/// An sRGB color.
pub type Rgb = [u8; 3];

/// What a terminal makes of a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Profile {
    /// Its name (`xterm-dark`).
    pub name: String,
    /// ANSI colors 0–15.
    pub palette: [Rgb; 16],
    /// The default glyph and background colors.
    pub fg: Rgb,
    pub bg: Rgb,
    /// Bold text in an ANSI color 0–7 shows its bright twin (8–15).
    pub bold_is_bright: bool,
    /// Which attributes it draws; one it lacks shows as plain text.
    pub italic: bool,
    pub strikethrough: bool,
    pub overline: bool,
    /// Underline styles beyond a single line (double, curly, dotted,
    /// dashed); without them every underline is single.
    pub underline_styles: bool,
}

const XTERM: [Rgb; 16] = [
    [0x00, 0x00, 0x00],
    [0xcd, 0x00, 0x00],
    [0x00, 0xcd, 0x00],
    [0xcd, 0xcd, 0x00],
    [0x00, 0x00, 0xee],
    [0xcd, 0x00, 0xcd],
    [0x00, 0xcd, 0xcd],
    [0xe5, 0xe5, 0xe5],
    [0x7f, 0x7f, 0x7f],
    [0xff, 0x00, 0x00],
    [0x00, 0xff, 0x00],
    [0xff, 0xff, 0x00],
    [0x5c, 0x5c, 0xff],
    [0xff, 0x00, 0xff],
    [0x00, 0xff, 0xff],
    [0xff, 0xff, 0xff],
];

const VGA: [Rgb; 16] = [
    [0x00, 0x00, 0x00],
    [0xaa, 0x00, 0x00],
    [0x00, 0xaa, 0x00],
    [0xaa, 0x55, 0x00],
    [0x00, 0x00, 0xaa],
    [0xaa, 0x00, 0xaa],
    [0x00, 0xaa, 0xaa],
    [0xaa, 0xaa, 0xaa],
    [0x55, 0x55, 0x55],
    [0xff, 0x55, 0x55],
    [0x55, 0xff, 0x55],
    [0xff, 0xff, 0x55],
    [0x55, 0x55, 0xff],
    [0xff, 0x55, 0xff],
    [0x55, 0xff, 0xff],
    [0xff, 0xff, 0xff],
];

const SOLARIZED: [Rgb; 16] = [
    [0x07, 0x36, 0x42],
    [0xdc, 0x32, 0x2f],
    [0x85, 0x99, 0x00],
    [0xb5, 0x89, 0x00],
    [0x26, 0x8b, 0xd2],
    [0xd3, 0x36, 0x82],
    [0x2a, 0xa1, 0x98],
    [0xee, 0xe8, 0xd5],
    [0x00, 0x2b, 0x36],
    [0xcb, 0x4b, 0x16],
    [0x58, 0x6e, 0x75],
    [0x65, 0x7b, 0x83],
    [0x83, 0x94, 0x96],
    [0x6c, 0x71, 0xc4],
    [0x93, 0xa1, 0xa1],
    [0xfd, 0xf6, 0xe3],
];

impl Profile {
    /// The profiles that ship: `xterm-dark` (the default), `xterm-light`,
    /// `console-16` (the VGA text console: bold brightens, no italic,
    /// strikethrough, overline or underline styles) and `solarized-light`
    /// (a low-contrast light theme).
    pub fn builtin() -> Vec<Profile> {
        let xterm = |name: &str, fg: Rgb, bg: Rgb| Profile {
            name: name.to_string(),
            palette: XTERM,
            fg,
            bg,
            bold_is_bright: false,
            italic: true,
            strikethrough: true,
            overline: true,
            underline_styles: true,
        };
        vec![
            xterm("xterm-dark", [0xe5, 0xe5, 0xe5], [0x00, 0x00, 0x00]),
            xterm("xterm-light", [0x00, 0x00, 0x00], [0xff, 0xff, 0xff]),
            Profile {
                name: "console-16".to_string(),
                palette: VGA,
                fg: VGA[7],
                bg: VGA[0],
                bold_is_bright: true,
                italic: false,
                strikethrough: false,
                overline: false,
                underline_styles: false,
            },
            Profile {
                name: "solarized-light".to_string(),
                palette: SOLARIZED,
                fg: [0x65, 0x7b, 0x83],
                bg: [0xfd, 0xf6, 0xe3],
                bold_is_bright: false,
                italic: true,
                strikethrough: true,
                overline: true,
                underline_styles: true,
            },
        ]
    }

    /// An app's own profile `name`: `palette` for ANSI colors 0–15 and
    /// `fg` / `bg` for the default colors, drawing every attribute and not
    /// brightening bold — change the fields for a terminal that differs.
    pub fn new(name: impl Into<String>, palette: [Rgb; 16], fg: Rgb, bg: Rgb) -> Profile {
        Profile {
            name: name.into(),
            palette,
            fg,
            bg,
            bold_is_bright: false,
            italic: true,
            strikethrough: true,
            overline: true,
            underline_styles: true,
        }
    }

    /// The built-in profile `name`.
    pub fn named(name: &str) -> Option<Profile> {
        Self::builtin().into_iter().find(|p| p.name == name)
    }

    /// The default profile, `xterm-dark`.
    pub fn default_profile() -> Profile {
        Self::named("xterm-dark").expect("xterm-dark ships")
    }

    /// The sRGB `color` shows as — the glyph's (`glyph`) or the
    /// background's — `bold` for a glyph's bright twin on a terminal that
    /// brightens bold.
    pub fn rgb(&self, color: ColorValue, glyph: bool, bold: bool) -> Rgb {
        match color {
            ColorValue::Default => {
                if glyph {
                    self.fg
                } else {
                    self.bg
                }
            }
            ColorValue::Ansi(n) => {
                let n = if glyph && bold && self.bold_is_bright && n < 8 {
                    n + 8
                } else {
                    n
                };
                indexed(&self.palette, n)
            }
            // The terminal is sent an alpha color's RGB, opaque
            // (`rdom_tui` SGR emission): that is what it shows.
            ColorValue::Rgb(r, g, b) | ColorValue::Rgba(r, g, b, _) => [r, g, b],
        }
    }
}

/// xterm-256 color `n`: the 16 of `palette`, the 6×6×6 cube, the 24 grays.
pub fn indexed(palette: &[Rgb; 16], n: u8) -> Rgb {
    match n {
        0..=15 => palette[usize::from(n)],
        16..=231 => {
            let i = n - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [level(i / 36), level((i / 6) % 6), level(i % 6)]
        }
        _ => {
            let v = 8 + 10 * (n - 232);
            [v, v, v]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Profile, indexed};
    use crate::capture::ColorValue;

    #[test]
    fn colors_resolve_through_the_profile() {
        let dark = Profile::default_profile();
        assert_eq!(dark.rgb(ColorValue::Default, false, false), [0, 0, 0]);
        assert_eq!(
            dark.rgb(ColorValue::Default, true, false),
            [0xe5, 0xe5, 0xe5]
        );
        assert_eq!(
            dark.rgb(ColorValue::Ansi(1), true, true),
            [0xcd, 0, 0],
            "xterm does not brighten bold"
        );
        let vga = Profile::named("console-16").unwrap();
        assert_eq!(
            vga.rgb(ColorValue::Ansi(1), true, true),
            [0xff, 0x55, 0x55],
            "the console does"
        );
        assert_eq!(indexed(&dark.palette, 196), [0xff, 0, 0]);
        assert_eq!(indexed(&dark.palette, 232), [8, 8, 8]);
        assert_eq!(
            dark.rgb(ColorValue::Rgba(255, 255, 255, 128), false, false),
            [255, 255, 255]
        );
        assert_eq!(Profile::builtin().len(), 4);
    }
}
