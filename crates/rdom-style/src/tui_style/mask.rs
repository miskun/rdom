//! [`MaskDeclarations`]: a style block's declarations of the mask
//! properties (CSS Masking 1 §6–§7), each kept as its CSS text — a cell
//! has no alpha to mask by, so none has a computed value or an effect.

use crate::Value;

/// The mask longhands a [`TuiStyle`](crate::TuiStyle) declares
/// ([`TuiStyle::masks`](crate::TuiStyle::masks)), each as CSS text (a
/// layer list comma-separated), `None` where the block does not declare
/// it. The `mask` and `mask-border` shorthands write them.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaskDeclarations {
    /// `mask-image`.
    pub mask_image: Option<Value<String>>,
    /// `mask-mode`.
    pub mask_mode: Option<Value<String>>,
    /// `mask-repeat`.
    pub mask_repeat: Option<Value<String>>,
    /// `mask-position`.
    pub mask_position: Option<Value<String>>,
    /// `mask-clip`.
    pub mask_clip: Option<Value<String>>,
    /// `mask-origin`.
    pub mask_origin: Option<Value<String>>,
    /// `mask-size`.
    pub mask_size: Option<Value<String>>,
    /// `mask-composite`.
    pub mask_composite: Option<Value<String>>,
    /// `mask-type`.
    pub mask_type: Option<Value<String>>,
    /// `mask-border-source`.
    pub mask_border_source: Option<Value<String>>,
    /// `mask-border-slice`.
    pub mask_border_slice: Option<Value<String>>,
    /// `mask-border-width`.
    pub mask_border_width: Option<Value<String>>,
    /// `mask-border-outset`.
    pub mask_border_outset: Option<Value<String>>,
    /// `mask-border-repeat`.
    pub mask_border_repeat: Option<Value<String>>,
    /// `mask-border-mode`.
    pub mask_border_mode: Option<Value<String>>,
}
