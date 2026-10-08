//! The table setters of the `TuiStyle` builder: `table-layout` (CSS 2.1
//! §17.5.2) and `caption-side` (§17.4.1).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{CaptionSide, TableLayout};

/// A setter for one [`TableDeclarations`](crate::TableDeclarations) field
/// and its `!important` twin.
macro_rules! table_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.table.$field = Some(Value::Specified(v));
            self
        }

        #[doc = concat!("Like `", stringify!($field), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: $ty) -> Self {
            self.important |= ImportantMask::$mask;
            self.$field(v)
        }
    };
}

impl TuiStyle {
    table_setter!(
        "table-layout",
        table_layout,
        table_layout_important,
        TABLE_LAYOUT,
        TableLayout
    );
    table_setter!(
        "caption-side",
        caption_side,
        caption_side_important,
        CAPTION_SIDE,
        CaptionSide
    );
}
