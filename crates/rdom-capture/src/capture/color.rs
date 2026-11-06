//! A painted color as the capture records it: typed in Rust, a short
//! string in JSON.
//!
//! | Color | JSON |
//! |---|---|
//! | the terminal's default (`Color::Reset`) | `"default"` |
//! | 24-bit | `"#rrggbb"` |
//! | 24-bit with alpha | `"#rrggbbaa"` |
//! | a palette index (`Color::Indexed`) | `"ansi:N"` (0–255) |

use std::fmt;

use rdom_tui::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A painted color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorValue {
    /// The terminal's default color.
    Default,
    /// 24-bit.
    Rgb(u8, u8, u8),
    /// 24-bit with an alpha below 255.
    Rgba(u8, u8, u8, u8),
    /// A palette index; the terminal's palette decides the color.
    Ansi(u8),
}

impl From<Color> for ColorValue {
    fn from(color: Color) -> Self {
        match color {
            Color::Reset => ColorValue::Default,
            Color::Indexed(n) => ColorValue::Ansi(n),
            Color::Rgb(r, g, b) => ColorValue::Rgb(r, g, b),
            Color::Rgba(r, g, b, a) => ColorValue::Rgba(r, g, b, a),
        }
    }
}

impl fmt::Display for ColorValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ColorValue::Default => f.write_str("default"),
            ColorValue::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
            ColorValue::Rgba(r, g, b, a) => write!(f, "#{r:02x}{g:02x}{b:02x}{a:02x}"),
            ColorValue::Ansi(n) => write!(f, "ansi:{n}"),
        }
    }
}

impl std::str::FromStr for ColorValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || format!("not a capture color: {s:?} (default, #rrggbb, #rrggbbaa or ansi:N)");
        if s == "default" {
            return Ok(ColorValue::Default);
        }
        if let Some(n) = s.strip_prefix("ansi:") {
            return n.parse().map(ColorValue::Ansi).map_err(|_| bad());
        }
        let hex = s.strip_prefix('#').ok_or_else(bad)?;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());
        match hex.len() {
            6 => Ok(ColorValue::Rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Ok(ColorValue::Rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => Err(bad()),
        }
    }
}

impl Serialize for ColorValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
