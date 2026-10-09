//! [`AnchorDeclarations`]: a style block's declarations of the anchor
//! positioning properties, the specified side of
//! [`AnchorStyle`](crate::layout::AnchorStyle).

use crate::Value;
use crate::layout::{
    AnchorName, AnchorScope, PositionAnchor, PositionArea, PositionTryOrder, PositionVisibility,
    TryFallback,
};

/// The CSS Anchor Positioning 1 properties a [`TuiStyle`](crate::TuiStyle)
/// declares ([`TuiStyle::anchor`](crate::TuiStyle::anchor)), one field per
/// longhand, `None` where the block does not declare it.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnchorDeclarations {
    /// `anchor-name` (§2.1).
    pub anchor_name: Option<Value<AnchorName>>,
    /// `anchor-scope` (§2.2).
    pub anchor_scope: Option<Value<AnchorScope>>,
    /// `position-anchor` (§2.3).
    pub position_anchor: Option<Value<PositionAnchor>>,
    /// `position-area` (§3.1); `Specified(None)` is `none`.
    pub position_area: Option<Value<PositionArea>>,
    /// `position-try-fallbacks` (§4.1); `Specified` empty is `none`.
    pub position_try_fallbacks: Option<Value<Vec<TryFallback>>>,
    /// `position-try-order` (§4.2).
    pub position_try_order: Option<Value<PositionTryOrder>>,
    /// `position-visibility` (§5).
    pub position_visibility: Option<Value<PositionVisibility>>,
}
