//! Scrollbar values: `scrollbar-gutter` (CSS Overflow 3 §3.3),
//! `scrollbar-width` (CSS Scrollbars 1 §3) and `scrollbar-color` (§2).

/// CSS `scrollbar-gutter: auto | stable && both-edges?` (CSS Overflow 3
/// §3.3) — whether a scroll container reserves its scrollbar's gutter
/// when no scrollbar shows. `auto`: only when the bar shows (an
/// `overflow: scroll` axis always, an `auto` one on overflow); `stable`:
/// always on an `overflow: hidden | scroll | auto` box, so content never
/// reflows when a bar appears; `both-edges`: on the opposite inline edge
/// too, whenever one is present.
///
/// Does not inherit. Initial value: `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarGutter {
    /// A gutter only where a scrollbar shows. CSS default.
    #[default]
    Auto,
    /// A gutter whenever the box can scroll on its block axis.
    Stable,
    /// `stable both-edges`: as `stable`, with a matching gutter on the
    /// opposite inline edge.
    StableBothEdges,
}

impl ScrollbarGutter {
    /// The value's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Stable => "stable",
            Self::StableBothEdges => "stable both-edges",
        }
    }

    /// `stable`, with or without `both-edges`.
    pub const fn is_stable(self) -> bool {
        matches!(self, Self::Stable | Self::StableBothEdges)
    }

    /// `both-edges`.
    pub const fn both_edges(self) -> bool {
        matches!(self, Self::StableBothEdges)
    }
}

/// CSS `scrollbar-width: auto | thin | none` (CSS Scrollbars 1 §3). Not
/// inherited. A terminal cell is the narrowest a bar can be, so `thin`
/// keeps the one-cell bar and draws it lighter (rdom's choice, DIVERGENCES
/// §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarWidth {
    /// The platform's bar (the initial value).
    #[default]
    Auto,
    /// A thinner bar: in a terminal, no track glyph and a light thumb.
    Thin,
    /// No bar and no gutter; the box still scrolls.
    None,
}

impl ScrollbarWidth {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Thin => "thin",
            Self::None => "none",
        }
    }
}

/// The platform's scrollbar colors — the `auto` of `scrollbar-color`,
/// and the colors of rdom's UA `::scrollbar` / `::scrollbar-thumb` rules:
/// a muted track glyph and a slightly lighter thumb glyph, no fill.
pub const NATIVE_SCROLLBAR_TRACK: crate::Color = crate::Color::Rgb(0x2D, 0x2F, 0x31);

/// The platform's scrollbar thumb color ([`NATIVE_SCROLLBAR_TRACK`]).
pub const NATIVE_SCROLLBAR_THUMB: crate::Color = crate::Color::Rgb(0x41, 0x43, 0x45);

/// CSS `scrollbar-color: auto | <color>{2}` (CSS Scrollbars 1 §2): the
/// thumb's color, then the track's. Inherited; the colors are kept as
/// specified (`currentcolor`, `var()`, `light-dark()` resolve where the
/// bar paints, against its element).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ScrollbarColor {
    /// The platform's colors (the initial value).
    #[default]
    Auto,
    /// The thumb and track colors.
    Colors {
        /// The thumb's color.
        thumb: crate::TuiColor,
        /// The track's color.
        track: crate::TuiColor,
    },
}
