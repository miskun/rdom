//! [`ScrollRange`]: a box's legal scroll offsets, read through
//! [`TuiAccessors::scroll_range`](super::TuiAccessors::scroll_range).

/// A box's legal scroll offsets per axis ([`TuiAccessors::scroll_range`](super::TuiAccessors::scroll_range)):
/// `scrollLeft` in [`x`](Self::x), `scrollTop` in [`y`](Self::y), each
/// `0 ..= overflow` or `-overflow ..= 0` (CSSOM View §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollRange {
    x: std::ops::RangeInclusive<i32>,
    y: std::ops::RangeInclusive<i32>,
}

impl ScrollRange {
    /// The range `x` horizontally and `y` vertically.
    pub fn new(x: std::ops::RangeInclusive<i32>, y: std::ops::RangeInclusive<i32>) -> Self {
        Self { x, y }
    }

    /// The legal `scrollLeft` values.
    pub fn x(&self) -> std::ops::RangeInclusive<i32> {
        self.x.clone()
    }

    /// The legal `scrollTop` values.
    pub fn y(&self) -> std::ops::RangeInclusive<i32> {
        self.y.clone()
    }
}
