//! CSS Fonts 4: the font properties. The terminal owns the font — one
//! monospaced face at one size — so `font-weight` and `font-style` are
//! drawn as SGR bold and italic, and the others parse, cascade and
//! serialize for the CSSOM without changing what is drawn.

/// `font-weight: normal | bold | bolder | lighter | <number [1,1000]>`
/// (CSS Fonts 4 §2.2). Inherited; initial `normal`. The keywords are kept
/// as written; the computed value is a [`Number`](Self::Number)
/// ([`FontWeight::computed`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontWeight {
    /// `normal`: 400.
    #[default]
    Normal,
    /// `bold`: 700.
    Bold,
    /// A weight in `[1, 1000]`.
    Number(f32),
    /// One step bolder than the parent's (§2.2's table).
    Bolder,
    /// One step lighter than the parent's.
    Lighter,
}

impl FontWeight {
    /// The weight as a number: `normal` 400, `bold` 700; `bolder` /
    /// `lighter` against `parent` (CSS Fonts 4 §2.2, "Determining the
    /// relative weight"), which is a computed weight.
    pub fn value(self, parent: f32) -> f32 {
        match self {
            FontWeight::Normal => 400.0,
            FontWeight::Bold => 700.0,
            FontWeight::Number(n) => n,
            FontWeight::Bolder => match parent {
                w if w < 350.0 => 400.0,
                w if w < 550.0 => 700.0,
                w if w < 900.0 => 900.0,
                w => w,
            },
            FontWeight::Lighter => match parent {
                w if w < 100.0 => w,
                w if w < 550.0 => 100.0,
                w if w < 750.0 => 400.0,
                _ => 700.0,
            },
        }
    }

    /// The computed value: a number, the relative keywords resolved
    /// against the parent's computed weight `parent`.
    pub fn computed(self, parent: f32) -> FontWeight {
        FontWeight::Number(self.value(parent))
    }

    /// Whether text this heavy is drawn bold (SGR 1): from 600 — the
    /// weights the CSS Fonts 4 §2.2 names "Semi Bold" and heavier.
    /// Lighter weights draw normal: the terminal's faint (SGR 2) dims the
    /// color rather than thinning the strokes (DIVERGENCES §2).
    pub fn is_bold(self, parent: f32) -> bool {
        self.value(parent) >= 600.0
    }
}

/// `font-style: normal | italic | oblique <angle [-90deg,90deg]>?` (CSS
/// Fonts 4 §2.4). Inherited; initial `normal`. Drawn as SGR italic, an
/// oblique with a non-zero angle included.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    /// `oblique`, its angle in degrees (`None`: the default, 14deg).
    Oblique(Option<f32>),
}

impl FontStyle {
    /// Whether text in this style is drawn italic (SGR 3): `italic`, and
    /// `oblique` at any angle but 0deg (an upright slant).
    pub fn is_italic(self) -> bool {
        match self {
            FontStyle::Normal => false,
            FontStyle::Italic => true,
            FontStyle::Oblique(angle) => angle != Some(0.0),
        }
    }
}

/// `font-size` (CSS Fonts 4 §2.5): parsed and kept; the terminal draws
/// one size. A length is a pixel length (`16px`, `1.2em` at 16px) or
/// rdom's cells.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FontSize {
    /// `<absolute-size>` (`xx-small` … `xxx-large`; `medium` the
    /// initial) or `<relative-size>` (`larger`, `smaller`), as written.
    Keyword(FontSizeKeyword),
    /// A `<length-percentage>`.
    Length(super::PaintLength),
    /// `math`.
    Math,
    /// `medium`, the initial value.
    #[default]
    Medium,
}

/// The `font-size` keywords other than `medium` (CSS Fonts 4 §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSizeKeyword {
    XxSmall,
    XSmall,
    Small,
    Large,
    XLarge,
    XxLarge,
    XxxLarge,
    Larger,
    Smaller,
}

impl FontSizeKeyword {
    /// Every keyword with its spelling.
    pub const ALL: [(FontSizeKeyword, &'static str); 9] = [
        (FontSizeKeyword::XxSmall, "xx-small"),
        (FontSizeKeyword::XSmall, "x-small"),
        (FontSizeKeyword::Small, "small"),
        (FontSizeKeyword::Large, "large"),
        (FontSizeKeyword::XLarge, "x-large"),
        (FontSizeKeyword::XxLarge, "xx-large"),
        (FontSizeKeyword::XxxLarge, "xxx-large"),
        (FontSizeKeyword::Larger, "larger"),
        (FontSizeKeyword::Smaller, "smaller"),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(k, _)| *k == self)
            .map_or("medium", |(_, s)| s)
    }
}

/// `font-family` (CSS Fonts 4 §2.1): the family names in order — each
/// a quoted string or a sequence of identifiers, generic families
/// (`monospace`, …) by name — or, set by the `font` shorthand, a system
/// font. Parsed and kept; the terminal's font is the one drawn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FontFamily {
    /// The initial value: the UA's default family.
    #[default]
    Initial,
    /// The listed families, each serialized as it is written back —
    /// shared, so a computed style inherits the list without copying it
    /// (C9G-PACKER-ALLOC).
    Names(std::sync::Arc<[String]>),
    /// A system font keyword of the `font` shorthand (§3.7: `caption`,
    /// `icon`, `menu`, `message-box`, `small-caption`, `status-bar`).
    System(SystemFont),
}

/// The `font` shorthand's system font keywords (CSS Fonts 4 §3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemFont {
    Caption,
    Icon,
    Menu,
    MessageBox,
    SmallCaption,
    StatusBar,
}

impl SystemFont {
    /// Every keyword with its spelling.
    pub const ALL: [(SystemFont, &'static str); 6] = [
        (SystemFont::Caption, "caption"),
        (SystemFont::Icon, "icon"),
        (SystemFont::Menu, "menu"),
        (SystemFont::MessageBox, "message-box"),
        (SystemFont::SmallCaption, "small-caption"),
        (SystemFont::StatusBar, "status-bar"),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(k, _)| *k == self)
            .map_or("caption", |(_, s)| s)
    }
}

/// `font-stretch` / `font-width` (CSS Fonts 4 §2.3): `normal`, a keyword
/// (`ultra-condensed` … `ultra-expanded`), or a percentage. Parsed and
/// kept; one width is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStretch {
    #[default]
    Normal,
    /// A width keyword other than `normal`.
    Keyword(FontStretchKeyword),
    /// A percentage of the normal width.
    Percent(f32),
}

/// The width keywords of `font-stretch` other than `normal` (CSS Fonts 4
/// §2.3), each standing for a percentage of the normal width. A closed
/// set: the grammar names these eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStretchKeyword {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl FontStretchKeyword {
    /// Every keyword with its spelling and percentage, narrowest first.
    pub const ALL: [(FontStretchKeyword, &'static str, f32); 8] = [
        (Self::UltraCondensed, "ultra-condensed", 50.0),
        (Self::ExtraCondensed, "extra-condensed", 62.5),
        (Self::Condensed, "condensed", 75.0),
        (Self::SemiCondensed, "semi-condensed", 87.5),
        (Self::SemiExpanded, "semi-expanded", 112.5),
        (Self::Expanded, "expanded", 125.0),
        (Self::ExtraExpanded, "extra-expanded", 150.0),
        (Self::UltraExpanded, "ultra-expanded", 200.0),
    ];

    fn entry(self) -> (FontStretchKeyword, &'static str, f32) {
        Self::ALL[self as usize]
    }

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        self.entry().1
    }

    /// The percentage of the normal width it stands for.
    pub fn percent(self) -> f32 {
        self.entry().2
    }

    /// The keyword spelled `s` (ASCII case-insensitive).
    pub fn from_keyword(s: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, k, _)| s.eq_ignore_ascii_case(k))
            .map(|(w, _, _)| *w)
    }
}

/// `font-variant` (CSS Fonts 4 §6.11) in CSS 2.1's form, which the `font`
/// shorthand takes: `normal | small-caps`. Parsed and kept; the terminal
/// draws no small capitals.
///
/// `#[non_exhaustive]`: an open vocabulary — CSS Fonts 4 adds
/// `all-small-caps`, `petite-caps`, `unicase`, … — that rdom takes in
/// part and draws none of, so a consumer meeting an unknown value can
/// treat it as `normal`:
///
/// ```compile_fail
/// use rdom_style::layout::FontVariant;
/// fn small(v: FontVariant) -> bool {
///     match v {
///         FontVariant::Normal => false,
///         FontVariant::SmallCaps => true,
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FontVariant {
    #[default]
    Normal,
    SmallCaps,
}

/// An element's computed font properties
/// ([`ComputedStyle::font`](crate::ComputedStyle::font)). All inherit, so
/// the cascade copies the group from the parent whole.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Font {
    /// `font-weight`, computed: a [`FontWeight::Number`].
    pub weight: FontWeight,
    /// `font-style`.
    pub style: FontStyle,
    /// `font-size`, as specified.
    pub size: FontSize,
    /// `font-family`.
    pub family: FontFamily,
    /// `font-stretch`.
    pub stretch: FontStretch,
    /// `font-variant`.
    pub variant: FontVariant,
}

impl Font {
    /// The computed weight as a number.
    pub fn weight(&self) -> f32 {
        self.weight.value(400.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CSS Fonts 4 §2.2's table of relative weights.
    #[test]
    fn bolder_and_lighter_follow_the_table() {
        for (parent, bolder, lighter) in [
            (50.0, 400.0, 50.0),
            (100.0, 400.0, 100.0),
            (300.0, 400.0, 100.0),
            (400.0, 700.0, 100.0),
            (600.0, 900.0, 400.0),
            (700.0, 900.0, 400.0),
            (800.0, 900.0, 700.0),
            (950.0, 950.0, 700.0),
        ] {
            assert_eq!(FontWeight::Bolder.value(parent), bolder, "{parent}");
            assert_eq!(FontWeight::Lighter.value(parent), lighter, "{parent}");
        }
        assert!(FontWeight::Number(600.0).is_bold(400.0));
        assert!(!FontWeight::Number(500.0).is_bold(400.0));
        assert!(!FontWeight::Lighter.is_bold(700.0));
        assert!(FontStyle::Oblique(None).is_italic());
        assert!(!FontStyle::Oblique(Some(0.0)).is_italic());
    }
}
