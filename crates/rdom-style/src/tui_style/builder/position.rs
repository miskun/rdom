//! The positioning setters of the `TuiStyle` builder: `position`, the
//! insets, `z-index`, `overlay`, `float`, `clear`.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;

impl TuiStyle {
    setter!(
        "position",
        position,
        position,
        position_important,
        POSITION,
        crate::layout::Position
    );
    setter!("top", top, top, top_important, TOP, crate::layout::Length);
    setter!(
        "right",
        right,
        right,
        right_important,
        RIGHT,
        crate::layout::Length
    );
    setter!(
        "bottom",
        bottom,
        bottom,
        bottom_important,
        BOTTOM,
        crate::layout::Length
    );
    setter!(
        "left",
        left,
        left,
        left_important,
        LEFT,
        crate::layout::Length
    );
    setter!(
        "z-index",
        z_index,
        z_index,
        z_index_important,
        Z_INDEX,
        crate::layout::ZIndex
    );
    setter!(
        "overlay",
        overlay,
        overlay,
        overlay_important,
        OVERLAY,
        crate::layout::Overlay
    );
    setter!(
        "float",
        float,
        float,
        float_important,
        FLOAT,
        crate::layout::Float
    );
    setter!(
        "clear",
        clear,
        clear,
        clear_important,
        CLEAR,
        crate::layout::Clear
    );
}
