//! The margin and padding setters of the `TuiStyle` builder (CSS Box 3
//! §3.2 / §4.2): the `margin` / `padding` shorthands, which write the
//! four longhands, and one setter per longhand.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{Margin, MarginValue, Padding, PaddingValue, Sides};

/// A setter for one spacing longhand and its `!important` twin:
/// `side_setter!("css-name", field, side, setter, important_setter,
/// MASK, ValueType)`.
macro_rules! side_setter {
    ($css:literal, $field:ident, $side:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` longhand to `v` (cells, or a `", stringify!($ty), "`), leaving the other sides as they are. Chainable.")]
        pub fn $setter(mut self, v: impl Into<$ty>) -> Self {
            self.$field.$side = Some(Value::Specified(v.into()));
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: impl Into<$ty>) -> Self {
            self.important |= ImportantMask::$mask;
            self.$setter(v)
        }
    };
}

impl TuiStyle {
    /// Set the `padding` shorthand: the four `padding-*` longhands.
    /// Accepts a `Padding` or a plain `u16` (`n` cells on every side).
    /// Chainable.
    pub fn padding(mut self, v: impl Into<Padding>) -> Self {
        self.padding = Sides::from(v.into()).map(|v| Some(Value::Specified(v)));
        self
    }
    /// Like `padding` but marks the four longhands `!important`.
    pub fn padding_important(mut self, v: impl Into<Padding>) -> Self {
        self.important |= ImportantMask::PADDING_TOP
            | ImportantMask::PADDING_RIGHT
            | ImportantMask::PADDING_BOTTOM
            | ImportantMask::PADDING_LEFT;
        self.padding(v)
    }
    /// Set the `margin` shorthand: the four `margin-*` longhands. Accepts
    /// a `Margin` or a plain `i16` (`n` cells on every side). Chainable.
    pub fn margin(mut self, v: impl Into<Margin>) -> Self {
        self.margin = Sides::from(v.into()).map(|v| Some(Value::Specified(v)));
        self
    }
    /// Like `margin` but marks the four longhands `!important`.
    pub fn margin_important(mut self, v: impl Into<Margin>) -> Self {
        self.important |= ImportantMask::MARGIN_TOP
            | ImportantMask::MARGIN_RIGHT
            | ImportantMask::MARGIN_BOTTOM
            | ImportantMask::MARGIN_LEFT;
        self.margin(v)
    }
    side_setter!(
        "margin-top",
        margin,
        top,
        margin_top,
        margin_top_important,
        MARGIN_TOP,
        MarginValue
    );
    side_setter!(
        "margin-right",
        margin,
        right,
        margin_right,
        margin_right_important,
        MARGIN_RIGHT,
        MarginValue
    );
    side_setter!(
        "margin-bottom",
        margin,
        bottom,
        margin_bottom,
        margin_bottom_important,
        MARGIN_BOTTOM,
        MarginValue
    );
    side_setter!(
        "margin-left",
        margin,
        left,
        margin_left,
        margin_left_important,
        MARGIN_LEFT,
        MarginValue
    );
    side_setter!(
        "padding-top",
        padding,
        top,
        padding_top,
        padding_top_important,
        PADDING_TOP,
        PaddingValue
    );
    side_setter!(
        "padding-right",
        padding,
        right,
        padding_right,
        padding_right_important,
        PADDING_RIGHT,
        PaddingValue
    );
    side_setter!(
        "padding-bottom",
        padding,
        bottom,
        padding_bottom,
        padding_bottom_important,
        PADDING_BOTTOM,
        PaddingValue
    );
    side_setter!(
        "padding-left",
        padding,
        left,
        padding_left,
        padding_left_important,
        PADDING_LEFT,
        PaddingValue
    );
}
