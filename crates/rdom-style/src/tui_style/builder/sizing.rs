//! The sizing setters of the `TuiStyle` builder: `width` / `height`,
//! their `min-*` / `max-*` (CSS Sizing 3 §5.2) and `aspect-ratio`.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::Size;

impl TuiStyle {
    /// `aspect-ratio: <w> / <h>`. Never fails: a `u16` term is finite and
    /// non-negative, and a zero term makes a degenerate ratio, which
    /// behaves as `auto` (CSS Sizing 4 §5.1). For fractional terms or
    /// `auto && <ratio>`, build an [`AspectRatio`](crate::layout::AspectRatio)
    /// or use the CSS parser.
    pub fn aspect_ratio(mut self, w: u16, h: u16) -> Self {
        let ratio = crate::layout::AspectRatio::new(f32::from(w), f32::from(h))
            .expect("u16 terms are finite and non-negative");
        self.aspect_ratio = Some(Value::Specified(Some(ratio)));
        self
    }
    /// `aspect-ratio` with `!important`.
    pub fn aspect_ratio_important(mut self, w: u16, h: u16) -> Self {
        let ratio = crate::layout::AspectRatio::new(f32::from(w), f32::from(h))
            .expect("u16 terms are finite and non-negative");
        self.aspect_ratio = Some(Value::Specified(Some(ratio)));
        self.important |= ImportantMask::ASPECT_RATIO;
        self
    }

    // Layout setters.
    // A flex weight is kept in `<number [0,∞]>` (`Size::validated`).
    /// Set the `width` property: the width of the box `box-sizing`
    /// names — the content box by default, the border box under
    /// `box-sizing: border-box` (CSS UI 3 §3.1). Accepts a `u16`
    /// (cells), a [`Size`] or an
    /// [`IntrinsicSize`](crate::layout::IntrinsicSize) keyword; a flex
    /// weight is kept in `<number [0,∞]>`. Chainable.
    pub fn width(mut self, v: impl Into<Size>) -> Self {
        self.width = Some(Value::Specified(v.into().validated()));
        self
    }
    /// Like `width` but marks the declaration `!important`.
    pub fn width_important(mut self, v: impl Into<Size>) -> Self {
        self.width = Some(Value::Specified(v.into().validated()));
        self.important |= ImportantMask::WIDTH;
        self
    }
    /// Set the `height` property: the height of the box `box-sizing`
    /// names — the content box by default, the border box under
    /// `box-sizing: border-box` (CSS UI 3 §3.1). Accepts a `u16`
    /// (cells), a [`Size`] or an
    /// [`IntrinsicSize`](crate::layout::IntrinsicSize) keyword; a flex
    /// weight is kept in `<number [0,∞]>`. Chainable.
    pub fn height(mut self, v: impl Into<Size>) -> Self {
        self.height = Some(Value::Specified(v.into().validated()));
        self
    }
    /// Like `height` but marks the declaration `!important`.
    pub fn height_important(mut self, v: impl Into<Size>) -> Self {
        self.height = Some(Value::Specified(v.into().validated()));
        self.important |= ImportantMask::HEIGHT;
        self
    }
    /// Set the `min-width` property. Accepts a `u16` (cells) or a
    /// [`MinSize`](crate::layout::MinSize). Chainable.
    pub fn min_width(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_width = Some(Value::Specified(v.into()));
        self
    }
    /// Like `min_width` but marks the declaration `!important`.
    pub fn min_width_important(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_width = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MIN_WIDTH;
        self
    }
    /// Set the `max-width` property. Accepts a `u16` (cells) or a
    /// [`MaxSize`](crate::layout::MaxSize). Chainable.
    pub fn max_width(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_width = Some(Value::Specified(v.into()));
        self
    }
    /// Like `max_width` but marks the declaration `!important`.
    pub fn max_width_important(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_width = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MAX_WIDTH;
        self
    }
    /// Set the `min-height` property. Accepts a `u16` (cells) or a
    /// [`MinSize`](crate::layout::MinSize). Chainable.
    pub fn min_height(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_height = Some(Value::Specified(v.into()));
        self
    }
    /// Like `min_height` but marks the declaration `!important`.
    pub fn min_height_important(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_height = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MIN_HEIGHT;
        self
    }
    /// Set the `max-height` property. Accepts a `u16` (cells) or a
    /// [`MaxSize`](crate::layout::MaxSize). Chainable.
    pub fn max_height(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_height = Some(Value::Specified(v.into()));
        self
    }
    /// Like `max_height` but marks the declaration `!important`.
    pub fn max_height_important(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_height = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MAX_HEIGHT;
        self
    }
}
