//! Overflow values (CSS Overflow 3 §3): `overflow` on each axis and the
//! `overflow-clip-margin` of a `clip` axis.

use super::VisualBox;

/// One axis of `overflow` (CSS Overflow 3 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// No clipping; content may draw outside the box.
    #[default]
    Visible,
    /// Clipped; scrollable; no scrollbar.
    Hidden,
    /// Clipped at the overflow clip edge (`overflow-clip-margin`); not a
    /// scroll container — no scrolling, even programmatic — and no
    /// formatting context of its own.
    Clip,
    /// Clipped; scrollable; scrollbar always visible.
    Scroll,
    /// Clipped; scrollable; scrollbar visible only when needed.
    Auto,
}

impl Overflow {
    /// Whether this value makes its box a scroll container (§3.1:
    /// `hidden`, `scroll` and `auto`).
    pub const fn is_scrollable(self) -> bool {
        matches!(self, Self::Hidden | Self::Scroll | Self::Auto)
    }

    /// Whether content past the box is clipped on this axis: any value
    /// but `visible`.
    pub const fn clips(self) -> bool {
        !matches!(self, Self::Visible)
    }

    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
            Self::Clip => "clip",
            Self::Scroll => "scroll",
            Self::Auto => "auto",
        }
    }
}

/// `overflow-clip-margin: <visual-box> || <length [0,∞]>` (CSS Overflow 3
/// §3.2): how far outside the named box a `clip` axis still paints. The
/// box is `padding-box` and the margin 0 when omitted; the margin is in
/// whole cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverflowClipMargin {
    /// The box the overflow clip edge starts from.
    pub visual_box: VisualBox,
    /// Cells the edge is outset by.
    pub margin: u16,
}

impl OverflowClipMargin {
    /// `visual_box` outset by `margin` cells.
    pub const fn new(visual_box: VisualBox, margin: u16) -> Self {
        Self { visual_box, margin }
    }
}

impl Default for OverflowClipMargin {
    /// The initial value, `0px`: the padding box (§3.2).
    fn default() -> Self {
        Self::new(VisualBox::PaddingBox, 0)
    }
}
