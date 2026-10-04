//! The UA palette — the colors the user-agent sheet paints its chrome
//! with, defined once so every rule agrees — and the CSS system colors
//! (CSS Color 4 §6.2), which name them.
//!
//! | System color | Value |
//! |---|---|
//! | `Canvas`, `ButtonFace` | the terminal's default background (`reset`) |
//! | `CanvasText`, `FieldText` | the terminal's default foreground (`reset`) |
//! | `LinkText`, `VisitedText`, `ActiveText`, `ButtonText`, `AccentColor`, `SelectedItem` | the UA accent, dodgerblue |
//! | `AccentColorText`, `SelectedItemText`, `MarkText` | black |
//! | `ButtonBorder` | the UA border gray `#3B4042` |
//! | `Field` | the UA field background `#1F2123` |
//! | `Highlight` / `HighlightText` | the UA selection, `#394B7E` / white |
//! | `Mark` | yellow |
//! | `GrayText` | the UA muted text `#7F868B` |
//!
//! Inside a color function, which needs a definite color, the terminal
//! defaults take the canvas model's values: black background and white
//! text under a dark color scheme, white and black under a light one.

use super::{Color, ColorScheme, named};

/// Field background tint — subtle dark warm gray. On dark
/// terminals it reads as a soft pillow under input/textarea text
/// (just visible enough to mark the field affordance). On light
/// terminals it's a dark rectangle.
pub(crate) const FIELD_BG: Color = Color::Rgb(0x1f, 0x21, 0x23);
/// Muted text — #7F868B. Cool gray that reads as supporting
/// prose against both light and dark surfaces. Used for
/// `:disabled`, placeholder, `<small>`, `<abbr>`, blockquote
/// text, scrollbar glyphs, helper text, etc.
pub(crate) const TEXT_MUTED: Color = Color::Rgb(0x7F, 0x86, 0x8B);
/// Default border — #3B4042. Subtle gray for box-drawing borders
/// and `<hr>` rules — distinct from `TEXT_MUTED` so a border
/// next to muted text still reads as chrome rather than as more
/// text.
pub(crate) const BORDER_DEFAULT: Color = Color::Rgb(0x3B, 0x40, 0x42);
/// Accent — dodgerblue (#1E90FF). Vivid on both light and dark
/// terminals. Replaces every former `ACCENT` use:
/// link fg, kbd, button fg, dialog border, focus glyph, select
/// chevron, range/progress bar accent, selected-option bg.
pub(crate) const ACCENT: Color = named::DODGERBLUE;
/// The selection background — a muted blue that keeps a one-cell
/// selection distinct from the caret.
pub(crate) const HIGHLIGHT: Color = Color::Rgb(0x39, 0x4B, 0x7E);

/// The canvas model: what the terminal's default background and text
/// count as where a definite color is needed — black and white under a
/// dark scheme, white and black under a light one.
pub fn canvas(scheme: ColorScheme) -> (Color, Color) {
    scheme.canvas()
}

/// A CSS system color (CSS Color 4 §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SystemColor {
    AccentColor,
    AccentColorText,
    ActiveText,
    ButtonBorder,
    ButtonFace,
    ButtonText,
    Canvas,
    CanvasText,
    Field,
    FieldText,
    GrayText,
    Highlight,
    HighlightText,
    LinkText,
    Mark,
    MarkText,
    SelectedItem,
    SelectedItemText,
    VisitedText,
}

impl SystemColor {
    /// Every system color, with its keyword.
    const ALL: [(SystemColor, &'static str); 19] = [
        (SystemColor::AccentColor, "accentcolor"),
        (SystemColor::AccentColorText, "accentcolortext"),
        (SystemColor::ActiveText, "activetext"),
        (SystemColor::ButtonBorder, "buttonborder"),
        (SystemColor::ButtonFace, "buttonface"),
        (SystemColor::ButtonText, "buttontext"),
        (SystemColor::Canvas, "canvas"),
        (SystemColor::CanvasText, "canvastext"),
        (SystemColor::Field, "field"),
        (SystemColor::FieldText, "fieldtext"),
        (SystemColor::GrayText, "graytext"),
        (SystemColor::Highlight, "highlight"),
        (SystemColor::HighlightText, "highlighttext"),
        (SystemColor::LinkText, "linktext"),
        (SystemColor::Mark, "mark"),
        (SystemColor::MarkText, "marktext"),
        (SystemColor::SelectedItem, "selecteditem"),
        (SystemColor::SelectedItemText, "selecteditemtext"),
        (SystemColor::VisitedText, "visitedtext"),
    ];

    /// The deprecated system colors (§6.2.1) and the color each maps to.
    const DEPRECATED: [(&'static str, SystemColor); 23] = [
        ("activeborder", SystemColor::ButtonBorder),
        ("activecaption", SystemColor::Canvas),
        ("appworkspace", SystemColor::Canvas),
        ("background", SystemColor::Canvas),
        ("buttonhighlight", SystemColor::ButtonFace),
        ("buttonshadow", SystemColor::ButtonFace),
        ("captiontext", SystemColor::CanvasText),
        ("inactiveborder", SystemColor::ButtonBorder),
        ("inactivecaption", SystemColor::Canvas),
        ("inactivecaptiontext", SystemColor::GrayText),
        ("infobackground", SystemColor::Canvas),
        ("infotext", SystemColor::CanvasText),
        ("menu", SystemColor::Canvas),
        ("menutext", SystemColor::CanvasText),
        ("scrollbar", SystemColor::Canvas),
        ("threeddarkshadow", SystemColor::ButtonBorder),
        ("threedface", SystemColor::ButtonFace),
        ("threedhighlight", SystemColor::ButtonBorder),
        ("threedlightshadow", SystemColor::ButtonBorder),
        ("threedshadow", SystemColor::ButtonBorder),
        ("window", SystemColor::Canvas),
        ("windowframe", SystemColor::ButtonBorder),
        ("windowtext", SystemColor::CanvasText),
    ];

    /// The system color a keyword names, ASCII case-insensitive; a
    /// deprecated keyword names the color it maps to (§6.2.1).
    pub fn from_keyword(name: &str) -> Option<SystemColor> {
        Self::ALL
            .iter()
            .find(|(_, k)| k.eq_ignore_ascii_case(name))
            .map(|(c, _)| *c)
            .or_else(|| {
                Self::DEPRECATED
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, c)| *c)
            })
    }

    /// The keyword, in lower case as the CSSOM serializes it.
    pub fn keyword(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(c, _)| *c == self)
            .map_or("canvastext", |(_, k)| k)
    }

    /// The color: the terminal's default background or foreground
    /// (`Color::Reset`) for the canvas colors, the UA palette for the
    /// rest (see the module table).
    pub fn color(self) -> Color {
        match self {
            SystemColor::Canvas
            | SystemColor::ButtonFace
            | SystemColor::CanvasText
            | SystemColor::FieldText => Color::Reset,
            SystemColor::AccentColor
            | SystemColor::ActiveText
            | SystemColor::ButtonText
            | SystemColor::LinkText
            | SystemColor::SelectedItem
            | SystemColor::VisitedText => ACCENT,
            SystemColor::AccentColorText
            | SystemColor::MarkText
            | SystemColor::SelectedItemText => named::BLACK,
            SystemColor::ButtonBorder => BORDER_DEFAULT,
            SystemColor::Field => FIELD_BG,
            SystemColor::GrayText => TEXT_MUTED,
            SystemColor::Highlight => HIGHLIGHT,
            SystemColor::HighlightText => named::WHITE,
            SystemColor::Mark => named::YELLOW,
        }
    }

    /// True for the colors that are the terminal's defaults, whose
    /// definite value depends on the color scheme.
    pub fn is_canvas(self) -> bool {
        self.color() == Color::Reset
    }

    /// The color as a definite sRGB color, for use inside a color
    /// function: the terminal defaults take the canvas model's values
    /// for `scheme` ([`ColorScheme::canvas`]).
    pub fn definite(self, scheme: ColorScheme) -> Color {
        let (background, text) = canvas(scheme);
        match self {
            SystemColor::Canvas | SystemColor::ButtonFace => background,
            SystemColor::CanvasText | SystemColor::FieldText => text,
            other => other.color(),
        }
    }
}
