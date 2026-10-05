//! The flex and box-alignment setters of the `TuiStyle` builder: the
//! gaps (CSS Box Alignment 3 §8), `flex-grow` / `flex-shrink` /
//! `flex-basis`, `flex-direction` and `flex-wrap`, the six alignment
//! properties, `order`, and the `display: flex` conveniences.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{Direction, Display};

impl TuiStyle {
    /// Set `gap`: whole cells (`gap(2)`) or a `calc()` / percentage
    /// (`gap(GapValue::from(CalcExpr::Percent(10.0)))`), resolved at
    /// layout. Chainable.
    pub fn gap(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        let v = v.into();
        self.row_gap = Some(Value::Specified(v.clone()));
        self.column_gap = Some(Value::Specified(v));
        self
    }
    /// Set `row-gap` (CSS Box Alignment 3 §8.1). Chainable.
    pub fn row_gap(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        self.row_gap = Some(Value::Specified(v.into()));
        self
    }
    /// Set `column-gap` (CSS Box Alignment 3 §8.1). Chainable.
    pub fn column_gap(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        self.column_gap = Some(Value::Specified(v.into()));
        self
    }

    /// Like `gap` but also marks the declaration `!important`.
    pub fn gap_important(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        self.important |= ImportantMask::ROW_GAP | ImportantMask::COLUMN_GAP;
        self.gap(v)
    }
    setter!(
        "flex-grow",
        flex_grow,
        flex_grow,
        flex_grow_important,
        FLEX_GROW,
        f32,
        crate::layout::valid_flex_factor
    );
    setter!(
        "flex-shrink",
        flex_shrink,
        flex_shrink,
        flex_shrink_important,
        FLEX_SHRINK,
        f32,
        crate::layout::valid_flex_factor
    );
    setter!(
        "flex-basis",
        flex_basis,
        flex_basis,
        flex_basis_important,
        FLEX_BASIS,
        crate::layout::FlexBasis
    );
    /// Set the `flex-direction` axis to `v`, not reversed (`row` /
    /// `column`). Chainable.
    pub fn direction(mut self, v: Direction) -> Self {
        self.direction = Some(Value::Specified(v));
        self.flex_reverse = Some(Value::Specified(false));
        self
    }
    /// Like `direction` but also marks the `flex-direction` declaration
    /// `!important`.
    pub fn direction_important(mut self, v: Direction) -> Self {
        self.important |= ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE;
        self.direction(v)
    }
    setter!(
        "justify-content",
        justify_content,
        justify_content,
        justify_content_important,
        JUSTIFY_CONTENT,
        crate::layout::Alignment
    );
    setter!(
        "justify-items",
        justify_items,
        justify_items,
        justify_items_important,
        JUSTIFY_ITEMS,
        crate::layout::Alignment
    );
    setter!(
        "justify-self",
        justify_self,
        justify_self,
        justify_self_important,
        JUSTIFY_SELF,
        crate::layout::Alignment
    );
    setter!(
        "align-content",
        align_content,
        align_content,
        align_content_important,
        ALIGN_CONTENT,
        crate::layout::Alignment
    );
    setter!(
        "align-items",
        align_items,
        align_items,
        align_items_important,
        ALIGN_ITEMS,
        crate::layout::Alignment
    );
    setter!(
        "align-self",
        align_self,
        align_self,
        align_self_important,
        ALIGN_SELF,
        crate::layout::Alignment
    );
    setter!(
        "flex-wrap",
        flex_wrap,
        flex_wrap,
        flex_wrap_important,
        FLEX_WRAP,
        crate::layout::FlexWrap
    );
    /// Set `flex-direction` to the reversed form of `v` (`row-reverse` /
    /// `column-reverse`, CSS Flexbox §5.1). Chainable.
    pub fn direction_reverse(mut self, v: Direction) -> Self {
        self.direction = Some(Value::Specified(v));
        self.flex_reverse = Some(Value::Specified(true));
        self
    }

    /// `display: flex` — outer [`Display::Block`] + inner
    /// [`Flow::Flex`](crate::layout::Flow::Flex). Mirrors the CSS
    /// `display: flex` keyword.
    ///
    /// Prefer this over `.display(Display::Block).flow(Flow::Flex)`:
    /// `.display(...)` *resets* `flow` (e.g. `Display::Block` forces
    /// `Flow::Block`), so `.flow(Flex).display(Block)` would silently
    /// clobber the flex flow. `.flex()` sets both in the right order.
    pub fn flex(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Block));
        self.flow = Some(Value::Specified(crate::layout::Flow::Flex));
        self.list_item = Some(Value::Specified(false));
        self
    }

    /// `display: flex; flex-direction: row`.
    pub fn flex_row(self) -> Self {
        self.flex().direction(crate::layout::Direction::Row)
    }

    /// `display: flex; flex-direction: column`.
    pub fn flex_column(self) -> Self {
        self.flex().direction(crate::layout::Direction::Column)
    }

    /// `display: inline-flex` — outer [`Display::Inline`] + inner
    /// [`Flow::Flex`](crate::layout::Flow::Flex).
    pub fn inline_flex(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Inline));
        self.flow = Some(Value::Specified(crate::layout::Flow::Flex));
        self.list_item = Some(Value::Specified(false));
        self
    }

    setter!("order", order, order, order_important, ORDER, i32);
}
