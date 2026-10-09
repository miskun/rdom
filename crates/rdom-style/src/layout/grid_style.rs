//! The grid properties of a computed style, one shared group
//! ([`ComputedStyle::grid`](crate::ComputedStyle::grid), C15G-STYLE-SIZE).

/// The computed grid properties (CSS Grid 2 §7–§8): a grid container's
/// explicit tracks, areas and implicit track sizes, a grid item's
/// placement.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq)]
pub struct GridStyle {
    /// `grid-template-columns` (CSS Grid 2 §7.2): the explicit columns of
    /// a grid container, viewport units resolved. Initial `none`.
    pub grid_template_columns: crate::layout::GridTemplate,
    /// `grid-template-rows` (§7.2): the explicit rows. Initial `none`.
    pub grid_template_rows: crate::layout::GridTemplate,
    /// `grid-template-areas` (CSS Grid 2 §7.3): the explicit grid's
    /// named areas. Initial `none`.
    pub grid_template_areas: crate::layout::GridTemplateAreas,
    /// `grid-auto-columns` (CSS Grid 2 §7.6): the implicit columns'
    /// sizes, repeated as a pattern; never empty. Initial `auto`, the
    /// shared [`TrackSize::AUTO_LIST`](crate::layout::TrackSize::AUTO_LIST)
    /// borrowed — every element starts from the initial style, so it
    /// allocates nothing (C7G-INITIAL-ALLOC); a declared list is owned.
    pub grid_auto_columns: std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    /// `grid-auto-rows` (§7.6): the implicit rows' sizes. Initial `auto`,
    /// borrowed as `grid_auto_columns`'.
    pub grid_auto_rows: std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    /// `grid-auto-flow` (CSS Grid 2 §7.7). Initial `row`.
    pub grid_auto_flow: crate::layout::GridAutoFlow,
    /// `grid-row-start` (CSS Grid 2 §8.3): where the item's grid area
    /// starts among the rows. Initial `auto`, as are the next three.
    pub grid_row_start: crate::layout::GridLine,
    /// `grid-row-end` (§8.3).
    pub grid_row_end: crate::layout::GridLine,
    /// `grid-column-start` (§8.3).
    pub grid_column_start: crate::layout::GridLine,
    /// `grid-column-end` (§8.3).
    pub grid_column_end: crate::layout::GridLine,
}

impl Default for GridStyle {
    fn default() -> Self {
        GridStyle {
            grid_template_columns: crate::layout::GridTemplate::None,
            grid_template_rows: crate::layout::GridTemplate::None,
            grid_template_areas: crate::layout::GridTemplateAreas::NONE,
            grid_auto_columns: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_rows: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_flow: crate::layout::GridAutoFlow::ROW,
            grid_row_start: crate::layout::GridLine::Auto,
            grid_row_end: crate::layout::GridLine::Auto,
            grid_column_start: crate::layout::GridLine::Auto,
            grid_column_end: crate::layout::GridLine::Auto,
        }
    }
}
