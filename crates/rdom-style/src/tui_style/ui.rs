//! [`UiDeclarations`]: a style block's declarations of the CSS UI 4
//! properties, the specified side of [`UiStyle`](crate::layout::UiStyle).

use crate::Value;
use crate::layout::{
    AccentColor, Appearance, BorderWidth, CaretAnimation, CaretShape, Cursor, OutlineColor,
    OutlineStyle, PaintLength,
};

/// The CSS Basic User Interface 4 properties a
/// [`TuiStyle`](crate::TuiStyle) declares
/// ([`TuiStyle::ui`](crate::TuiStyle::ui)), one field per longhand,
/// `None` where the block does not declare it. The `outline` shorthand
/// writes the first three.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiDeclarations {
    /// `outline-style` (CSS UI 4 §5.2).
    pub outline_style: Option<Value<OutlineStyle>>,
    /// `outline-width` (§5.3).
    pub outline_width: Option<Value<BorderWidth>>,
    /// `outline-color` (§5.3).
    pub outline_color: Option<Value<OutlineColor>>,
    /// `outline-offset` (§5.4).
    pub outline_offset: Option<Value<PaintLength>>,
    /// `cursor` (§4.1).
    pub cursor: Option<Value<Cursor>>,
    /// `caret-shape` (§6.2.2); the `caret` shorthand writes it.
    pub caret_shape: Option<Value<CaretShape>>,
    /// `caret-animation` (§6.2.1); the `caret` shorthand writes it.
    pub caret_animation: Option<Value<CaretAnimation>>,
    /// `accent-color` (§6.3).
    pub accent_color: Option<Value<AccentColor>>,
    /// `appearance` (§7.1), and its legacy name `-webkit-appearance`.
    pub appearance: Option<Value<Appearance>>,
}
