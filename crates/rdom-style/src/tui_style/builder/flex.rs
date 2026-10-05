//! The flex and box-alignment setters of the `TuiStyle` builder: the
//! gaps (CSS Box Alignment 3 §8), `flex-grow` / `flex-shrink` /
//! `flex-basis`, `flex-direction` and `flex-wrap`, the six alignment
//! properties, `order`, and the `display: flex` conveniences.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{AlignProperty, Alignment, Direction, Display, FlexDirection};

/// A Box Alignment setter and its `!important` twin:
/// `align_setter!("css-name", field, setter, important_setter, MASK,
/// Property)`. The value is checked against the property's grammar
/// ([`checked`]).
macro_rules! align_setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident, $prop:ident) => {
        #[doc = concat!("Set `", $css, "` to `v` — a keyword (`Align::Center`) or an [`Alignment`] (`Alignment::safe(Align::End)`). Chainable. A value outside `", $css, "`'s grammar ([`Alignment::is_valid_for`]) is refused: a debug build panics, a release build leaves the declaration unset, as a CSS parser drops it.")]
        pub fn $setter(mut self, v: impl Into<Alignment>) -> Self {
            if let Some(v) = checked(v.into(), AlignProperty::$prop) {
                self.$field = Some(Value::Specified(v));
            }
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: impl Into<Alignment>) -> Self {
            if let Some(v) = checked(v.into(), AlignProperty::$prop) {
                self.$field = Some(Value::Specified(v));
                self.important |= ImportantMask::$mask;
            }
            self
        }
    };
}

/// `v` when `property`'s grammar takes it. An out-of-grammar value is a
/// programming error — loud in a debug build — and, as CSS drops an
/// invalid declaration, sets nothing in a release one.
fn checked(v: Alignment, property: AlignProperty) -> Option<Alignment> {
    let ok = v.is_valid_for(property);
    debug_assert!(
        ok,
        "`{}` is not a value of `{}` (CSS Box Alignment 3)",
        crate::parse::values::serialize_alignment(v),
        property.name()
    );
    ok.then_some(v)
}

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
    align_setter!(
        "justify-content",
        justify_content,
        justify_content,
        justify_content_important,
        JUSTIFY_CONTENT,
        JustifyContent
    );
    align_setter!(
        "justify-items",
        justify_items,
        justify_items,
        justify_items_important,
        JUSTIFY_ITEMS,
        JustifyItems
    );
    align_setter!(
        "justify-self",
        justify_self,
        justify_self,
        justify_self_important,
        JUSTIFY_SELF,
        JustifySelf
    );
    align_setter!(
        "align-content",
        align_content,
        align_content,
        align_content_important,
        ALIGN_CONTENT,
        AlignContent
    );
    align_setter!(
        "align-items",
        align_items,
        align_items,
        align_items_important,
        ALIGN_ITEMS,
        AlignItems
    );
    align_setter!(
        "align-self",
        align_self,
        align_self,
        align_self_important,
        ALIGN_SELF,
        AlignSelf
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
    /// Like `direction_reverse` but also marks the `flex-direction`
    /// declaration `!important`.
    pub fn direction_reverse_important(mut self, v: Direction) -> Self {
        self.important |= ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE;
        self.direction_reverse(v)
    }
    /// Set `flex-direction` to `v` (CSS Flexbox §5.1): its axis and
    /// whether it is reversed, in one value. Chainable.
    pub fn flex_direction(self, v: FlexDirection) -> Self {
        if v.is_reversed() {
            self.direction_reverse(v.axis())
        } else {
            self.direction(v.axis())
        }
    }
    /// Like `flex_direction` but also marks the declaration `!important`.
    pub fn flex_direction_important(mut self, v: FlexDirection) -> Self {
        self.important |= ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE;
        self.flex_direction(v)
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

    /// Set `place-content` (CSS Box Alignment 3 §5.5): `align-content` to
    /// `align` and `justify-content` to `justify`. Chainable; each value
    /// checked as its longhand's builder checks it (a debug build panics on
    /// a value outside the grammar, a release build leaves that longhand
    /// unset). CSS's one-value form is both arguments the same, except that
    /// a baseline value gives `justify-content: start`.
    pub fn place_content(self, align: impl Into<Alignment>, justify: impl Into<Alignment>) -> Self {
        self.align_content(align).justify_content(justify)
    }

    /// Set `place-items` (§6.4): `align-items` to `align` and
    /// `justify-items` to `justify`, each checked as
    /// [`place_content`](Self::place_content)'s are. `place-items: center`
    /// is `place_items(Align::Center, Align::Center)`.
    pub fn place_items(self, align: impl Into<Alignment>, justify: impl Into<Alignment>) -> Self {
        self.align_items(align).justify_items(justify)
    }

    /// Set `place-self` (§6.5): `align-self` to `align` and `justify-self`
    /// to `justify`, each checked as [`place_content`](Self::place_content)'s
    /// are.
    pub fn place_self(self, align: impl Into<Alignment>, justify: impl Into<Alignment>) -> Self {
        self.align_self(align).justify_self(justify)
    }
}
