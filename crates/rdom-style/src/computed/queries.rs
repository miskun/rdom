//! `ComputedStyle`'s derived queries: one answer read from several
//! computed fields (the flex direction, scroll-container-ness, atomic
//! inline-ness, clipping) and the overflow normalization of CSS
//! Overflow 3 §3.1.

use super::ComputedStyle;

impl ComputedStyle {
    /// `flex-direction` as one value (CSS Flexbox §5.1): the axis
    /// ([`direction`](Self::direction)) and [`flex_reverse`](Self::flex_reverse).
    pub fn flex_direction(&self) -> crate::layout::FlexDirection {
        crate::layout::FlexDirection::new(self.direction, self.flex_reverse)
    }

    /// Whether the box is a scroll container (CSS Overflow 3 §3.1):
    /// `overflow` `hidden`, `scroll` or `auto` on an axis. A `clip` box
    /// clips without being one.
    pub fn is_scroll_container(&self) -> bool {
        self.overflow_x.is_scrollable() || self.overflow_y.is_scrollable()
    }

    /// Whether the box is an atomic inline (CSS Display 3 §2.4): an
    /// inline-level box that is no inline box — `inline-block`, `inline
    /// flow-root`, `inline-flex`, `inline-grid` — laid out as one unit in
    /// its line.
    pub fn is_atomic_inline(&self) -> bool {
        use crate::layout::{Display, Flow};
        self.display == Display::InlineBlock
            || (self.display == Display::Inline && self.flow != Flow::Block)
    }

    /// Whether the box clips its content on either axis — a scroll
    /// container or an `overflow: clip` axis.
    pub fn clips_overflow(&self) -> bool {
        self.overflow_x.clips() || self.overflow_y.clips()
    }

    /// CSS Overflow 3 §3.1's computed value: beside an axis that makes a
    /// scroll container, `visible` computes to `auto` and `clip` to
    /// `hidden`; otherwise both stay as specified.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn normalize_overflow(&mut self) {
        use crate::layout::Overflow;
        if !self.is_scroll_container() {
            return;
        }
        for axis in [&mut self.overflow_x, &mut self.overflow_y] {
            *axis = match *axis {
                Overflow::Visible => Overflow::Auto,
                Overflow::Clip => Overflow::Hidden,
                other @ (Overflow::Hidden | Overflow::Scroll | Overflow::Auto) => other,
            };
        }
    }
}
