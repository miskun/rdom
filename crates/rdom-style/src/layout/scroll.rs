//! Scrolling values: `overscroll-behavior` (CSS Overscroll Behavior 1
//! §3) and `scroll-padding` (CSS Scroll Snap 1 §4.1).

/// `overscroll-behavior-x` / `-y`: `contain | none | auto` (CSS
/// Overscroll Behavior 1 §3) — what a scroll container does with a
/// scroll it cannot take on that axis. Not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverscrollBehavior {
    /// The scroll chains to the nearest scrollable ancestor (the initial
    /// value).
    #[default]
    Auto,
    /// No chaining to ancestors; the box's own overscroll affordance
    /// (none in a terminal).
    Contain,
    /// No chaining and no overscroll affordance.
    None,
}

impl OverscrollBehavior {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Contain => "contain",
            Self::None => "none",
        }
    }

    /// Whether a scroll this box cannot take goes on to its scrollable
    /// ancestor.
    pub const fn chains(self) -> bool {
        matches!(self, Self::Auto)
    }
}

/// One side of `scroll-padding`: `auto | <length-percentage [0,∞]>` (CSS
/// Scroll Snap 1 §4.1) — how far the scroll container's optimal viewing
/// region is inset from its scrollport's edge. A percentage is of the
/// scrollport's size on the side's axis. Not inherited.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ScrollPadding {
    /// The UA's choice — 0 in rdom (the initial value).
    #[default]
    Auto,
    /// A length or percentage.
    Length(crate::layout::PaddingValue),
}

impl ScrollPadding {
    /// The inset in cells for a scrollport `port` cells long on this
    /// side's axis.
    pub fn resolve(&self, port: u16) -> u16 {
        match self {
            Self::Auto => 0,
            Self::Length(v) => v.resolve(port),
        }
    }
}
