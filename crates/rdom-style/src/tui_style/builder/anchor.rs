//! The anchor positioning setters of the `TuiStyle` builder (CSS Anchor
//! Positioning 1).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{
    AnchorName, AnchorScope, PositionAnchor, PositionArea, PositionTryOrder, PositionVisibility,
    TryFallback,
};

/// A setter for one [`AnchorDeclarations`](crate::AnchorDeclarations)
/// field and its `!important` twin.
macro_rules! anchor_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.anchor.$field = Some(Value::Specified(v));
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
    anchor_setter!(
        "anchor-name",
        anchor_name,
        anchor_name_important,
        ANCHOR_NAME,
        AnchorName
    );
    anchor_setter!(
        "anchor-scope",
        anchor_scope,
        anchor_scope_important,
        ANCHOR_SCOPE,
        AnchorScope
    );
    anchor_setter!(
        "position-anchor",
        position_anchor,
        position_anchor_important,
        POSITION_ANCHOR,
        PositionAnchor
    );
    anchor_setter!(
        "position-area",
        position_area,
        position_area_important,
        POSITION_AREA,
        Option<PositionArea>
    );
    anchor_setter!(
        "position-try-fallbacks",
        position_try_fallbacks,
        position_try_fallbacks_important,
        POSITION_TRY_FALLBACKS,
        Vec<TryFallback>
    );
    anchor_setter!(
        "position-try-order",
        position_try_order,
        position_try_order_important,
        POSITION_TRY_ORDER,
        PositionTryOrder
    );
    anchor_setter!(
        "position-visibility",
        position_visibility,
        position_visibility_important,
        POSITION_VISIBILITY,
        PositionVisibility
    );
}
