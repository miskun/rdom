//! Line clamping values (CSS Overflow 4 §4): `block-ellipsis`,
//! `continue`, and the legacy `-webkit-box-orient` that, with `display:
//! -webkit-box`, makes `continue: -webkit-legacy` take effect.

/// `block-ellipsis: no-ellipsis | auto | <string>` (§4.3): what the last
/// line before a clamp point ends with. Inherited.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BlockEllipsis {
    /// No marker (the initial value).
    #[default]
    NoEllipsis,
    /// `…` (U+2026).
    Auto,
    /// The string.
    Str(String),
}

impl BlockEllipsis {
    /// The marker the last line takes, `None` for `no-ellipsis`.
    pub fn marker(&self) -> Option<&str> {
        match self {
            Self::NoEllipsis => None,
            Self::Auto => Some("\u{2026}"),
            Self::Str(s) => Some(s),
        }
    }
}

/// `continue: auto | discard | collapse | -webkit-legacy` (§4.4): what
/// happens to the content after a clamp point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Continue {
    /// No clamp (the initial value).
    #[default]
    Auto,
    /// The content after the clamp point is discarded (a fragmentation
    /// break in CSS; rdom does as `collapse`, DIVERGENCES).
    Discard,
    /// The content after the clamp point is hidden and the box's
    /// automatic height ends at it.
    Collapse,
    /// `collapse`, when the box is `display: -webkit-box` /
    /// `-webkit-inline-box` with `-webkit-box-orient: vertical`.
    WebkitLegacy,
}

impl Continue {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Discard => "discard",
            Self::Collapse => "collapse",
            Self::WebkitLegacy => "-webkit-legacy",
        }
    }
}

/// `-webkit-box-orient: horizontal | vertical | inline-axis | block-axis`
/// (the legacy flexbox's axis, Compat Standard): read only as the
/// condition of `continue: -webkit-legacy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxOrient {
    /// `horizontal`.
    Horizontal,
    /// `vertical`.
    Vertical,
    /// `inline-axis` (the initial value).
    #[default]
    InlineAxis,
    /// `block-axis`.
    BlockAxis,
}

impl BoxOrient {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::InlineAxis => "inline-axis",
            Self::BlockAxis => "block-axis",
        }
    }

    /// Whether the axis is vertical (`horizontal-tb`: `vertical` and
    /// `block-axis`).
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::BlockAxis)
    }
}
