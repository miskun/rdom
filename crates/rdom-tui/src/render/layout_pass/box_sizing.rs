//! `box-sizing` (CSS UI 3 §3.1, now CSS Sizing 3 "Box Edges for
//! Sizing"): the one conversion between the box `width` / `height` and
//! their `min-*` / `max-*` measure and the border box the layout pass
//! stores in `TuiExt::layout`.
//!
//! Every site that turns a declared size into cells goes through a
//! [`Sizer`] for its axis, so content-box and border-box sizing — and
//! the border-box floor, where the content box never goes below zero —
//! are decided here once.

use crate::layout::{BoxSizing, Direction};
use crate::style::ComputedStyle;

/// A box's sizing along one axis: its `box-sizing` and its padding plus
/// border ("chrome") on that axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sizer {
    sizing: BoxSizing,
    chrome: u16,
}

impl Sizer {
    /// The horizontal axis of `computed`. Padding percentages resolve
    /// against the containing block's width `cb_width` (CSS Box 3 §4.2).
    pub(crate) fn horizontal(computed: &ComputedStyle, cb_width: u16) -> Self {
        let b = &computed.border;
        Self {
            sizing: computed.box_sizing,
            chrome: computed
                .padding
                .horizontal(cb_width)
                .saturating_add(b.left.cells() + b.right.cells()),
        }
    }

    /// The vertical axis of `computed`; padding percentages still
    /// resolve against the containing block's *width* (CSS Box 3 §4.2).
    pub(crate) fn vertical(computed: &ComputedStyle, cb_width: u16) -> Self {
        let b = &computed.border;
        Self {
            sizing: computed.box_sizing,
            chrome: computed
                .padding
                .vertical(cb_width)
                .saturating_add(b.top.cells() + b.bottom.cells()),
        }
    }

    /// The axis a size along `direction` measures: `Row` is the width.
    pub(crate) fn along(computed: &ComputedStyle, direction: Direction, cb_width: u16) -> Self {
        match direction {
            Direction::Row => Self::horizontal(computed, cb_width),
            Direction::Column => Self::vertical(computed, cb_width),
        }
    }

    /// Padding plus border on this axis.
    pub(crate) fn chrome(self) -> u16 {
        self.chrome
    }

    /// The border-box size a declared size of `cells` gives: the chrome
    /// added under `content-box`; under `border-box` the size itself,
    /// floored at the chrome — "the content width … floored at 0" (CSS
    /// UI 3 §3.1).
    pub(crate) fn outer(self, cells: u16) -> u16 {
        match self.sizing {
            BoxSizing::ContentBox => cells.saturating_add(self.chrome),
            BoxSizing::BorderBox => cells.max(self.chrome),
        }
    }

    /// [`outer`](Self::outer) of a resolved `min-*` / `max-*` /
    /// `width` / `height`, `None` (unresolved, `none`) kept.
    pub(crate) fn outer_opt(self, cells: Option<u16>) -> Option<u16> {
        cells.map(|c| self.outer(c))
    }

    /// The content-box size a declared size of `cells` gives: the size
    /// under `content-box`, the size less the chrome (at least 0) under
    /// `border-box`.
    pub(crate) fn inner(self, cells: u16) -> u16 {
        match self.sizing {
            BoxSizing::ContentBox => cells,
            BoxSizing::BorderBox => cells.saturating_sub(self.chrome),
        }
    }

    /// [`inner`](Self::inner) of an optional size.
    pub(crate) fn inner_opt(self, cells: Option<u16>) -> Option<u16> {
        cells.map(|c| self.inner(c))
    }

    /// A used border-box size, floored at the chrome: whatever sized
    /// the box (stretch, `auto`, a flex distribution), the content box
    /// is never negative (CSS 2.1 §10.2, CSS Flexbox §9.7).
    pub(crate) fn floor(self, outer: u16) -> u16 {
        outer.max(self.chrome)
    }

    /// Whether the box's declared sizes measure its content box.
    pub(crate) fn is_content_box(self) -> bool {
        self.sizing == BoxSizing::ContentBox
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Border, Padding};

    fn style(sizing: BoxSizing) -> ComputedStyle {
        ComputedStyle {
            box_sizing: sizing,
            padding: Padding::new(1, 2, 1, 2),
            border: Border::single(),
            ..ComputedStyle::initial()
        }
    }

    /// CSS UI 3 §3.1: content-box adds the chrome; border-box keeps the
    /// size and floors it at the chrome.
    #[test]
    fn outer_and_inner_per_box_sizing() {
        let c = Sizer::horizontal(&style(BoxSizing::ContentBox), 80);
        assert_eq!(c.chrome(), 6);
        assert_eq!((c.outer(10), c.inner(10)), (16, 10));
        let b = Sizer::horizontal(&style(BoxSizing::BorderBox), 80);
        assert_eq!((b.outer(10), b.inner(10)), (10, 4));
        assert_eq!((b.outer(3), b.inner(3)), (6, 0));
        let v = Sizer::vertical(&style(BoxSizing::BorderBox), 80);
        assert_eq!((v.chrome(), v.outer(0), v.floor(1)), (4, 4, 4));
    }

    /// Saturating: a huge size plus chrome stays in `u16`.
    #[test]
    fn outer_saturates() {
        let c = Sizer::horizontal(&style(BoxSizing::ContentBox), 80);
        assert_eq!(c.outer(u16::MAX), u16::MAX);
    }
}
