//! Overflow values (CSS Overflow 3 §3, Overflow 4 §3): `overflow` on each
//! axis, the `overflow-clip-margin` of a `clip` axis and `text-overflow`.

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

/// One edge's value of `text-overflow` (CSS Overflow 4 §3). `fade` and
/// `fade()` (a sub-cell gradient) are not values here (DIVERGENCES).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TextOverflowSide {
    /// Cut the content at the edge.
    #[default]
    Clip,
    /// Hide whole characters to fit `…` (U+2026) at the edge.
    Ellipsis,
    /// Hide whole characters to fit the string at the edge.
    Str(String),
}

impl TextOverflowSide {
    /// The marker painted at the edge, `None` for `clip`.
    pub fn marker(&self) -> Option<&str> {
        match self {
            Self::Clip => None,
            Self::Ellipsis => Some("\u{2026}"),
            Self::Str(s) => Some(s),
        }
    }
}

/// `text-overflow: [ clip | ellipsis | <string> ]{1,2}` (CSS Overflow 4
/// §3): one value for the end line box edge (the start edge clips), or
/// two for the line-left and the line-right edge.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextOverflow {
    first: TextOverflowSide,
    second: Option<TextOverflowSide>,
}

impl TextOverflow {
    /// One value: the end edge's.
    pub fn one(end: TextOverflowSide) -> Self {
        Self {
            first: end,
            second: None,
        }
    }

    /// Two values: the line-left edge's, then the line-right edge's.
    pub fn two(left: TextOverflowSide, right: TextOverflowSide) -> Self {
        Self {
            first: left,
            second: Some(right),
        }
    }

    /// The values as written: the first, and the second when given.
    pub fn values(&self) -> (&TextOverflowSide, Option<&TextOverflowSide>) {
        (&self.first, self.second.as_ref())
    }

    /// `(line-left, line-right)` for a block whose `direction` is `rtl`
    /// or not: two values as written; one value at the end edge — the
    /// right under `ltr`, the left under `rtl` — beside a `clip` start.
    pub fn line_sides(&self, rtl: bool) -> (&TextOverflowSide, &TextOverflowSide) {
        const CLIP: &TextOverflowSide = &TextOverflowSide::Clip;
        match (&self.second, rtl) {
            (Some(right), _) => (&self.first, right),
            (None, false) => (CLIP, &self.first),
            (None, true) => (&self.first, CLIP),
        }
    }

    /// Whether both edges clip — the initial value's behaviour.
    pub fn is_clip(&self) -> bool {
        self.first == TextOverflowSide::Clip
            && self
                .second
                .as_ref()
                .is_none_or(|s| *s == TextOverflowSide::Clip)
    }
}
