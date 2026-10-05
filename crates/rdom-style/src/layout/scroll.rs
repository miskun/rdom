//! Scrolling values: `overscroll-behavior` (CSS Overscroll Behavior 1
//! §3), `scroll-padding` (CSS Scroll Snap 1 §4.1) and the snapping
//! properties (§5–§6).

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

/// The axes a snap container snaps on (CSS Scroll Snap 1 §5.1):
/// `x | y | block | inline | both`. In `horizontal-tb` — rdom's only
/// writing mode — `block` is `y` and `inline` is `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollSnapAxis {
    X,
    Y,
    Block,
    Inline,
    Both,
}

impl ScrollSnapAxis {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Block => "block",
            Self::Inline => "inline",
            Self::Both => "both",
        }
    }

    /// Whether it snaps on the horizontal and the vertical axis, in
    /// `horizontal-tb`.
    pub const fn physical(self) -> (bool, bool) {
        match self {
            Self::X | Self::Inline => (true, false),
            Self::Y | Self::Block => (false, true),
            Self::Both => (true, true),
        }
    }
}

/// How strictly a snap container snaps (CSS Scroll Snap 1 §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollSnapStrictness {
    /// It may rest only at a snap position.
    Mandatory,
    /// It snaps when a snap position is near where a scroll ends (the
    /// default).
    #[default]
    Proximity,
}

/// `scroll-snap-type: none | [x | y | block | inline | both] [mandatory |
/// proximity]?` (CSS Scroll Snap 1 §5.1). Not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollSnapType {
    /// Not a snap container (the initial value).
    #[default]
    None,
    /// A snap container on `axis`, with `strictness`.
    Snap(ScrollSnapAxis, ScrollSnapStrictness),
}

/// One axis of `scroll-snap-align` (CSS Scroll Snap 1 §6.1): which edge
/// of the snap area aligns with the snapport's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SnapAlign {
    /// No snap position on this axis (the initial value).
    #[default]
    None,
    Start,
    End,
    Center,
}

impl SnapAlign {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Start => "start",
            Self::End => "end",
            Self::Center => "center",
        }
    }
}

/// `scroll-snap-align: [none | start | end | center]{1,2}` (CSS Scroll
/// Snap 1 §6.1): the block axis's alignment, then the inline axis's (one
/// value is both). Not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollSnapAlign {
    /// The block axis's (vertical in `horizontal-tb`).
    pub block: SnapAlign,
    /// The inline axis's (horizontal).
    pub inline: SnapAlign,
}

/// `scroll-snap-stop: normal | always` (CSS Scroll Snap 1 §6.2): whether
/// a scroll may pass this box's snap positions. Not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollSnapStop {
    /// It may (the initial value).
    #[default]
    Normal,
    /// It may not.
    Always,
}
