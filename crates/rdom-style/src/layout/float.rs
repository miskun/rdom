//! Float values (CSS 2.1 §9.5): `float` and `clear`, with the
//! flow-relative keywords of CSS Logical 1 §2.3.

/// `float: none | left | right | inline-start | inline-end` (CSS 2.1
/// §9.5.1, CSS Logical 1 §2.3). Not inherited; computes as specified —
/// except to `none` on an absolutely positioned box (CSS 2.1 §9.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Float {
    /// Not floated (the initial value).
    #[default]
    None,
    /// Floated to the left.
    Left,
    /// Floated to the right.
    Right,
    /// Floated to the inline-start side of the containing block.
    InlineStart,
    /// Floated to the inline-end side of the containing block.
    InlineEnd,
}

/// The physical side a float is placed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSide {
    /// The containing block's left edge.
    Left,
    /// The containing block's right edge.
    Right,
}

impl Float {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Left => "left",
            Self::Right => "right",
            Self::InlineStart => "inline-start",
            Self::InlineEnd => "inline-end",
        }
    }

    /// The side the box floats to in a containing block whose
    /// `direction` is `rtl` when `rtl` (CSS Logical 1 §2.3); `None` when
    /// it does not float.
    pub const fn side(self, rtl: bool) -> Option<FloatSide> {
        match (self, rtl) {
            (Self::None, _) => None,
            (Self::Left, _) | (Self::InlineStart, false) | (Self::InlineEnd, true) => {
                Some(FloatSide::Left)
            }
            (Self::Right, _) | (Self::InlineStart, true) | (Self::InlineEnd, false) => {
                Some(FloatSide::Right)
            }
        }
    }
}

/// `clear: none | left | right | both | inline-start | inline-end` (CSS
/// 2.1 §9.5.2, CSS Logical 1 §2.3). Not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clear {
    /// No clearance (the initial value).
    #[default]
    None,
    /// Below the earlier left floats.
    Left,
    /// Below the earlier right floats.
    Right,
    /// Below every earlier float.
    Both,
    /// Below the earlier floats on the containing block's inline-start
    /// side.
    InlineStart,
    /// Below the earlier floats on its inline-end side.
    InlineEnd,
}

impl Clear {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Left => "left",
            Self::Right => "right",
            Self::Both => "both",
            Self::InlineStart => "inline-start",
            Self::InlineEnd => "inline-end",
        }
    }

    /// Which sides' floats the box clears, `(left, right)`, in a
    /// containing block whose `direction` is `rtl` when `rtl`.
    pub const fn sides(self, rtl: bool) -> (bool, bool) {
        match (self, rtl) {
            (Self::None, _) => (false, false),
            (Self::Both, _) => (true, true),
            (Self::Left, _) | (Self::InlineStart, false) | (Self::InlineEnd, true) => (true, false),
            (Self::Right, _) | (Self::InlineStart, true) | (Self::InlineEnd, false) => {
                (false, true)
            }
        }
    }
}
