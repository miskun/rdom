//! Color schemes (CSS Color Adjust 1 §2, CSS Color 5 §5): the
//! `color-scheme` property's value, the scheme an element uses, and
//! the rule that reads a scheme off a terminal's background.

use crate::Color;
use crate::parse::token::Token;

/// A color scheme: light (dark text on a light background) or dark.
/// rdom falls back to dark, the common terminal default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorScheme {
    Light,
    #[default]
    Dark,
}

impl ColorScheme {
    /// The scheme a terminal background calls for: light when black
    /// text on it has more contrast than white text (WCAG 2 contrast
    /// ratio, from the background's relative luminance). The terminal
    /// default and palette colors count as dark.
    pub fn for_background(background: Color) -> ColorScheme {
        let (r, g, b) = match background {
            Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
            Color::Indexed(n) => super::palette::xterm_rgb(n),
            Color::Reset => return ColorScheme::Dark,
        };
        let lin = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
        // Contrast with black, (y + 0.05) / 0.05, against contrast with
        // white, 1.05 / (y + 0.05).
        if (y + 0.05) * (y + 0.05) > 1.05 * 0.05 {
            ColorScheme::Light
        } else {
            ColorScheme::Dark
        }
    }

    /// The canvas model: `(background, text)` — what the terminal's
    /// default colors count as where a definite color is needed
    /// (compositing, color functions): black and white when dark, white
    /// and black when light.
    pub fn canvas(self) -> (Color, Color) {
        match self {
            ColorScheme::Dark => (Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255)),
            ColorScheme::Light => (Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0)),
        }
    }

    /// The keyword.
    pub fn keyword(self) -> &'static str {
        match self {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        }
    }
}

/// The value of `color-scheme` (CSS Color Adjust 1 §2): `normal`, or
/// the schemes an element supports in order of preference — `light`,
/// `dark`, or custom identifiers, which name no scheme rdom knows and
/// are kept for serialization — and the `only` flag. Inherited.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ColorSchemeList {
    /// Empty for `normal`. `light` / `dark` in lower case, custom
    /// identifiers as written.
    names: Vec<String>,
    only: bool,
}

impl ColorSchemeList {
    /// `normal`: the element takes the document's preferred scheme.
    pub fn normal() -> Self {
        Self::default()
    }

    /// `[light | dark]+` from `schemes`, in order.
    pub fn of(schemes: &[ColorScheme]) -> Self {
        ColorSchemeList {
            names: schemes.iter().map(|s| s.keyword().to_string()).collect(),
            only: false,
        }
    }

    /// Parse `normal | [ light | dark | <custom-ident> ]+ && only?`.
    pub fn parse(value: &[Token]) -> Option<ColorSchemeList> {
        if let [Token::Ident(n)] = value
            && n.eq_ignore_ascii_case("normal")
        {
            return Some(Self::normal());
        }
        let mut out = ColorSchemeList::default();
        for (i, token) in value.iter().enumerate() {
            let Token::Ident(name) = token else {
                return None;
            };
            let lower = name.to_ascii_lowercase();
            match lower.as_str() {
                // `only` comes first or last, once.
                "only" if !out.only && (i == 0 || i == value.len() - 1) => out.only = true,
                "light" | "dark" => out.names.push(lower),
                // Not a `<custom-ident>` here (CSS Values 4 §6.2).
                "only" | "normal" | "initial" | "inherit" | "unset" | "revert" | "revert-layer"
                | "default" => return None,
                _ => out.names.push(name.clone()),
            }
        }
        (!out.names.is_empty()).then_some(out)
    }

    /// True for `normal`.
    pub fn is_normal(&self) -> bool {
        self.names.is_empty()
    }

    /// The scheme the element uses (§2.1) when the document prefers
    /// `preferred`: that one if the element supports it or says
    /// `normal`, else the first scheme it supports; an element that
    /// names only custom schemes behaves as `normal`.
    pub fn used(&self, preferred: ColorScheme) -> ColorScheme {
        let supported = |s: ColorScheme| self.names.iter().any(|n| n == s.keyword());
        if supported(preferred) {
            return preferred;
        }
        self.names
            .iter()
            .find_map(|n| match n.as_str() {
                "light" => Some(ColorScheme::Light),
                "dark" => Some(ColorScheme::Dark),
                _ => None,
            })
            .unwrap_or(preferred)
    }

    /// The CSS text: `normal`, or the names then `only`.
    pub fn to_css(&self) -> String {
        if self.is_normal() {
            return "normal".to_string();
        }
        let mut out = self.names.join(" ");
        if self.only {
            out.push_str(" only");
        }
        out
    }
}

#[cfg(test)]
#[path = "scheme_tests.rs"]
mod tests;
