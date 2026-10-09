//! The containment and container setters of the `TuiStyle` builder (CSS
//! Containment 2, CSS Conditional 5, CSS Will Change 1): `contain`,
//! `content-visibility`, `contain-intrinsic-*`, `will-change`,
//! `container-type` and `container-name`.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;

impl TuiStyle {
    setter!(
        "contain",
        contain,
        contain,
        contain_important,
        CONTAIN,
        crate::layout::Contain
    );
    setter!(
        "content-visibility",
        content_visibility,
        content_visibility,
        content_visibility_important,
        CONTENT_VISIBILITY,
        crate::layout::ContentVisibility
    );
    setter!(
        "will-change",
        will_change,
        will_change,
        will_change_important,
        WILL_CHANGE,
        crate::layout::WillChange
    );
    setter!(
        "container-type",
        container_type,
        container_type,
        container_type_important,
        CONTAINER_TYPE,
        crate::layout::ContainerType
    );
    setter!(
        "container-name",
        container_name,
        container_name,
        container_name_important,
        CONTAINER_NAME,
        crate::layout::ContainerName
    );
    setter!(
        "contain-intrinsic-width",
        contain_intrinsic_width,
        contain_intrinsic_width,
        contain_intrinsic_width_important,
        CONTAIN_INTRINSIC_WIDTH,
        crate::layout::ContainIntrinsicSize
    );
    setter!(
        "contain-intrinsic-height",
        contain_intrinsic_height,
        contain_intrinsic_height,
        contain_intrinsic_height_important,
        CONTAIN_INTRINSIC_HEIGHT,
        crate::layout::ContainIntrinsicSize
    );
}
