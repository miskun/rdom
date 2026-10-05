//! The grid setters of the `TuiStyle` builder (CSS Grid Layout 2): the
//! explicit track lists.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::GridTemplate;

/// A track-list setter and its `!important` twin:
/// `template_setter!("css-name", field, setter, important_setter, MASK)`.
/// A value outside the grammar ([`GridTemplate::is_valid`]) is refused.
macro_rules! template_setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident) => {
        #[doc = concat!("Set `", $css, "` to `v` — `GridTemplate::None`, a `TrackList`, or a `Vec<TrackSize>`. Chainable. A list outside the grammar ([`GridTemplate::is_valid`]: an `fr` minimum, an automatic repetition beside a non-fixed size, …) is refused: a debug build panics, a release build leaves the declaration unset, as a CSS parser drops it.")]
        pub fn $setter(mut self, v: impl Into<GridTemplate>) -> Self {
            if let Some(v) = checked(v.into(), $css) {
                self.$field = Some(Value::Specified(v));
            }
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: impl Into<GridTemplate>) -> Self {
            if let Some(v) = checked(v.into(), $css) {
                self.$field = Some(Value::Specified(v));
                self.important |= ImportantMask::$mask;
            }
            self
        }
    };
}

/// `v` when it is in the grammar. An invalid list is a programming
/// error — loud in a debug build — and sets nothing in a release one.
fn checked(v: GridTemplate, property: &str) -> Option<GridTemplate> {
    let ok = v.is_valid();
    debug_assert!(
        ok,
        "`{}` is not a value of `{property}` (CSS Grid 2 §7.2)",
        crate::parse::values::serialize_grid_template(&v),
    );
    ok.then_some(v)
}

impl TuiStyle {
    template_setter!(
        "grid-template-columns",
        grid_template_columns,
        grid_template_columns,
        grid_template_columns_important,
        GRID_TEMPLATE_COLUMNS
    );
    template_setter!(
        "grid-template-rows",
        grid_template_rows,
        grid_template_rows,
        grid_template_rows_important,
        GRID_TEMPLATE_ROWS
    );
}
