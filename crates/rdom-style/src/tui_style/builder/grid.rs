//! The grid setters of the `TuiStyle` builder (CSS Grid Layout 2): the
//! grid container conveniences, the explicit track lists and named
//! areas, the implicit
//! track sizes, `grid-auto-flow`, and the items' placement.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{Display, Flow, GridLine, GridTemplate, TrackSize};

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

/// An implicit-track-sizes setter and its `!important` twin:
/// `auto_setter!("css-name", field, setter, important_setter, MASK)`.
/// A list outside `<track-size>+` (empty, or a size
/// [`TrackSize::is_valid`] refuses) is refused.
macro_rules! auto_setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident) => {
        #[doc = concat!("Set `", $css, "` (CSS Grid 2 §7.6) to `sizes`, the implicit tracks' sizes repeated as a pattern. Chainable. An empty list or a size outside the grammar (an `fr` minimum) is refused: a debug build panics, a release build leaves the declaration unset.")]
        pub fn $setter(mut self, sizes: impl IntoIterator<Item = TrackSize>) -> Self {
            if let Some(v) = checked_sizes(sizes.into_iter().collect(), $css) {
                self.$field = Some(Value::Specified(v));
            }
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, sizes: impl IntoIterator<Item = TrackSize>) -> Self {
            if let Some(v) = checked_sizes(sizes.into_iter().collect(), $css) {
                self.$field = Some(Value::Specified(v));
                self.important |= ImportantMask::$mask;
            }
            self
        }
    };
}

/// `sizes` when it is a `<track-size>+`; refused as [`checked`] refuses.
fn checked_sizes(sizes: Vec<TrackSize>, property: &str) -> Option<Vec<TrackSize>> {
    let ok = !sizes.is_empty() && sizes.iter().all(TrackSize::is_valid);
    debug_assert!(
        ok,
        "`{}` is not a value of `{property}` (CSS Grid 2 §7.6)",
        crate::parse::values::serialize_track_sizes(&sizes),
    );
    ok.then_some(sizes)
}

/// A placement-longhand setter and its `!important` twin:
/// `line_setter!("css-name", field, setter, important_setter, MASK)`.
/// A line outside `<grid-line>` ([`GridLine::is_valid`]) is refused.
macro_rules! line_setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident) => {
        #[doc = concat!("Set `", $css, "` (CSS Grid 2 §8.3) to `line`. Chainable. A line outside the grammar (line `0`, a span below one, a name `span` / `auto`) is refused: a debug build panics, a release build leaves the declaration unset.")]
        pub fn $setter(mut self, line: GridLine) -> Self {
            if let Some(v) = checked_line(line, $css) {
                self.$field = Some(Value::Specified(v));
            }
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, line: GridLine) -> Self {
            if let Some(v) = checked_line(line, $css) {
                self.$field = Some(Value::Specified(v));
                self.important |= ImportantMask::$mask;
            }
            self
        }
    };
}

/// `line` when it is a `<grid-line>`; refused as [`checked`] refuses.
fn checked_line(line: GridLine, property: &str) -> Option<GridLine> {
    let ok = line.is_valid();
    debug_assert!(
        ok,
        "`{}` is not a value of `{property}` (CSS Grid 2 §8.3)",
        crate::parse::values::serialize_grid_line(&line),
    );
    ok.then_some(line)
}

impl TuiStyle {
    /// `display: grid` — outer [`Display::Block`] + inner [`Flow::Grid`]
    /// (CSS Display 3 §2.7, CSS Grid 2 §5.1): a block-level grid
    /// container. Sets both halves, as `.flex()` does.
    pub fn grid(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Block));
        self.flow = Some(Value::Specified(Flow::Grid));
        self.list_item = Some(Value::Specified(false));
        self
    }

    /// `display: inline-grid` — outer [`Display::Inline`] + inner
    /// [`Flow::Grid`]: an inline-level grid container, an atomic inline.
    pub fn inline_grid(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Inline));
        self.flow = Some(Value::Specified(Flow::Grid));
        self.list_item = Some(Value::Specified(false));
        self
    }

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
    setter!(
        "grid-template-areas",
        grid_template_areas,
        grid_template_areas,
        grid_template_areas_important,
        GRID_TEMPLATE_AREAS,
        crate::layout::GridTemplateAreas
    );
    auto_setter!(
        "grid-auto-columns",
        grid_auto_columns,
        grid_auto_columns,
        grid_auto_columns_important,
        GRID_AUTO_COLUMNS
    );
    auto_setter!(
        "grid-auto-rows",
        grid_auto_rows,
        grid_auto_rows,
        grid_auto_rows_important,
        GRID_AUTO_ROWS
    );
    setter!(
        "grid-auto-flow",
        grid_auto_flow,
        grid_auto_flow,
        grid_auto_flow_important,
        GRID_AUTO_FLOW,
        crate::layout::GridAutoFlow
    );
    line_setter!(
        "grid-row-start",
        grid_row_start,
        grid_row_start,
        grid_row_start_important,
        GRID_ROW_START
    );
    line_setter!(
        "grid-row-end",
        grid_row_end,
        grid_row_end,
        grid_row_end_important,
        GRID_ROW_END
    );
    line_setter!(
        "grid-column-start",
        grid_column_start,
        grid_column_start,
        grid_column_start_important,
        GRID_COLUMN_START
    );
    line_setter!(
        "grid-column-end",
        grid_column_end,
        grid_column_end,
        grid_column_end_important,
        GRID_COLUMN_END
    );

    /// Set `grid-row` (CSS Grid 2 §8.4): `grid-row-start` and
    /// `grid-row-end`. Chainable; each line checked as its longhand's.
    pub fn grid_row(self, start: GridLine, end: GridLine) -> Self {
        self.grid_row_start(start).grid_row_end(end)
    }

    /// Set `grid-column` (§8.4): `grid-column-start` and
    /// `grid-column-end`. Chainable.
    pub fn grid_column(self, start: GridLine, end: GridLine) -> Self {
        self.grid_column_start(start).grid_column_end(end)
    }

    /// Set `grid-area` (§8.4): row-start, column-start, row-end,
    /// column-end, in the shorthand's order. Chainable.
    pub fn grid_area(
        self,
        row_start: GridLine,
        column_start: GridLine,
        row_end: GridLine,
        column_end: GridLine,
    ) -> Self {
        self.grid_row(row_start, row_end)
            .grid_column(column_start, column_end)
    }
}
