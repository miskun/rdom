//! The multi-column (CSS Multi-column 1) and fragmentation (CSS
//! Fragmentation 3) setters of the `TuiStyle` builder. `column-gap`
//! (`builder/flex.rs`) is shared with flex and grid.

use super::super::{ImportantMask, TuiStyle};
use crate::layout::{
    BorderStyle, BorderWidth, BoxDecorationBreak, BreakBetween, BreakInside, ColumnCount,
    ColumnFill, ColumnSpan, ColumnWidth,
};
use crate::{TuiColor, Value};

/// A setter for one field of a declaration group and its `!important`
/// twin, as `setter!` is for a `TuiStyle` field.
macro_rules! group_setter {
    ($group:ident, $css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.$group.$field = Some(Value::Specified(v));
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
    group_setter!(
        multicol,
        "column-count",
        column_count,
        column_count_important,
        COLUMN_COUNT,
        ColumnCount
    );
    group_setter!(
        multicol,
        "column-width",
        column_width,
        column_width_important,
        COLUMN_WIDTH,
        ColumnWidth
    );
    group_setter!(
        multicol,
        "column-rule-style",
        column_rule_style,
        column_rule_style_important,
        COLUMN_RULE_STYLE,
        BorderStyle
    );
    group_setter!(
        multicol,
        "column-rule-width",
        column_rule_width,
        column_rule_width_important,
        COLUMN_RULE_WIDTH,
        BorderWidth
    );
    group_setter!(
        multicol,
        "column-rule-color",
        column_rule_color,
        column_rule_color_important,
        COLUMN_RULE_COLOR,
        TuiColor
    );
    group_setter!(
        multicol,
        "column-span",
        column_span,
        column_span_important,
        COLUMN_SPAN,
        ColumnSpan
    );
    group_setter!(
        multicol,
        "column-fill",
        column_fill,
        column_fill_important,
        COLUMN_FILL,
        ColumnFill
    );
    group_setter!(
        fragmentation,
        "break-before",
        break_before,
        break_before_important,
        BREAK_BEFORE,
        BreakBetween
    );
    group_setter!(
        fragmentation,
        "break-after",
        break_after,
        break_after_important,
        BREAK_AFTER,
        BreakBetween
    );
    group_setter!(
        fragmentation,
        "break-inside",
        break_inside,
        break_inside_important,
        BREAK_INSIDE,
        BreakInside
    );
    group_setter!(
        fragmentation,
        "orphans",
        orphans,
        orphans_important,
        ORPHANS,
        u32
    );
    group_setter!(
        fragmentation,
        "widows",
        widows,
        widows_important,
        WIDOWS,
        u32
    );
    group_setter!(
        fragmentation,
        "box-decoration-break",
        box_decoration_break,
        box_decoration_break_important,
        BOX_DECORATION_BREAK,
        BoxDecorationBreak
    );
}
