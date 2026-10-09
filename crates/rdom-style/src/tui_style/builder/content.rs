//! The generated content, list and counter setters of the `TuiStyle`
//! builder (CSS Generated Content 3, CSS Lists 3): `content`, the
//! `list-style-*` longhands, `marker-side`, `quotes` and the `counter-*`
//! properties.

use super::super::{ImportantMask, TuiStyle};
use crate::{Content, Value};

impl TuiStyle {
    setter!(
        "counter-reset",
        counter_reset,
        counter_reset,
        counter_reset_important,
        COUNTER_RESET,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        "counter-increment",
        counter_increment,
        counter_increment,
        counter_increment_important,
        COUNTER_INCREMENT,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        "counter-set",
        counter_set,
        counter_set,
        counter_set_important,
        COUNTER_SET,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        "content",
        content,
        content,
        content_important,
        CONTENT,
        Content
    );
    setter!(
        "list-style-type",
        list_style_type,
        list_style_type,
        list_style_type_important,
        LIST_STYLE_TYPE,
        crate::layout::ListStyleType
    );
    setter!(
        "list-style-position",
        list_style_position,
        list_style_position,
        list_style_position_important,
        LIST_STYLE_POSITION,
        crate::layout::ListStylePosition
    );
    setter!(
        "list-style-image",
        list_style_image,
        list_style_image,
        list_style_image_important,
        LIST_STYLE_IMAGE,
        crate::layout::ListStyleImage
    );
    setter!(
        "marker-side",
        marker_side,
        marker_side,
        marker_side_important,
        MARKER_SIDE,
        crate::layout::MarkerSide
    );
    setter!(
        "quotes",
        quotes,
        quotes,
        quotes_important,
        QUOTES,
        crate::Quotes
    );
}
