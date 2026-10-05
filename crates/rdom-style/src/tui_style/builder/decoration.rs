//! The background, border and shadow setters of the `TuiStyle`
//! builder (CSS Backgrounds 3).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{Border, BorderRadius, BorderWidth, BoxShadow, Corners, Sides};

impl TuiStyle {
    /// Set the four `border-*-style`s. Chainable.
    pub fn border(mut self, border: Border) -> Self {
        self.border_style = border.sides().map(|s| Some(Value::Specified(s)));
        self
    }
    pub fn border_important(mut self, border: Border) -> Self {
        self.border_style = border.sides().map(|s| Some(Value::Specified(s)));
        self.important |= ImportantMask::BORDER_TOP_STYLE
            | ImportantMask::BORDER_RIGHT_STYLE
            | ImportantMask::BORDER_BOTTOM_STYLE
            | ImportantMask::BORDER_LEFT_STYLE;
        self
    }
    /// Set the four `border-*-radius`es: one [`BorderRadius`] for every
    /// corner (`BorderRadius::cells(1.0)` rounds them all), or a
    /// [`Corners`] of them. Chainable.
    pub fn border_radius(mut self, radius: impl Into<Corners<BorderRadius>>) -> Self {
        self.border_radius = radius.into().map(|r| Some(Value::Specified(r)));
        self
    }
    /// Like `border_radius` but also marks the four `border-*-radius`
    /// declarations `!important`.
    pub fn border_radius_important(mut self, radius: impl Into<Corners<BorderRadius>>) -> Self {
        self.border_radius = radius.into().map(|r| Some(Value::Specified(r)));
        self.important |= ImportantMask::BORDER_TOP_LEFT_RADIUS
            | ImportantMask::BORDER_TOP_RIGHT_RADIUS
            | ImportantMask::BORDER_BOTTOM_RIGHT_RADIUS
            | ImportantMask::BORDER_BOTTOM_LEFT_RADIUS;
        self
    }
    // Backgrounds (CSS Backgrounds 3 §3), one entry per layer. Only the
    // clip of the final layer has an effect; the rest are inert.
    setter!(
        "background-image",
        background_image,
        background_image,
        background_image_important,
        BACKGROUND_IMAGE,
        Vec<String>
    );
    setter!(
        "background-position",
        background_position,
        background_position,
        background_position_important,
        BACKGROUND_POSITION,
        Vec<String>
    );
    setter!(
        "background-size",
        background_size,
        background_size,
        background_size_important,
        BACKGROUND_SIZE,
        Vec<String>
    );
    setter!(
        "background-repeat",
        background_repeat,
        background_repeat,
        background_repeat_important,
        BACKGROUND_REPEAT,
        Vec<crate::layout::BackgroundRepeat>
    );
    setter!(
        "background-attachment",
        background_attachment,
        background_attachment,
        background_attachment_important,
        BACKGROUND_ATTACHMENT,
        Vec<crate::layout::BackgroundAttachment>
    );
    setter!(
        "background-origin",
        background_origin,
        background_origin,
        background_origin_important,
        BACKGROUND_ORIGIN,
        Vec<crate::layout::VisualBox>
    );
    setter!(
        "background-clip",
        background_clip,
        background_clip,
        background_clip_important,
        BACKGROUND_CLIP,
        Vec<crate::layout::VisualBox>
    );
    /// Set `border-width` on all four sides. Chainable.
    pub fn border_width(mut self, width: BorderWidth) -> Self {
        self.border_width = Sides::all(Some(Value::Specified(width)));
        self
    }
    pub fn border_width_important(mut self, width: BorderWidth) -> Self {
        self.border_width = Sides::all(Some(Value::Specified(width)));
        self.important |= ImportantMask::BORDER_TOP_WIDTH
            | ImportantMask::BORDER_RIGHT_WIDTH
            | ImportantMask::BORDER_BOTTOM_WIDTH
            | ImportantMask::BORDER_LEFT_WIDTH;
        self
    }

    setter!(
        "box-shadow",
        box_shadow,
        box_shadow,
        box_shadow_important,
        BOX_SHADOW,
        Vec<BoxShadow>
    );
    setter!(
        "border-spacing",
        border_spacing,
        border_spacing,
        border_spacing_important,
        BORDER_SPACING,
        crate::layout::BorderSpacing
    );
}
