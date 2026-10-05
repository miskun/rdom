//! Scrolling values: `overscroll-behavior` (CSS Overscroll Behavior 1
//! §3).

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
