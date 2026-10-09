//! The grid declarations of a declaration block, one shared group
//! ([`TuiStyle::grid`](crate::TuiStyle::grid), C15G-STYLE-SIZE).

use crate::Value;

/// The declared grid properties (CSS Grid 2 §7–§8).
///
/// Closed (DESIGN): a new field fails a destructuring pattern.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridDeclarations {
    /// `grid-template-columns` (CSS Grid 2 §7.2): the explicit grid's
    /// columns.
    pub grid_template_columns: Option<Value<crate::layout::GridTemplate>>,
    /// `grid-template-rows` (§7.2): the explicit grid's rows.
    pub grid_template_rows: Option<Value<crate::layout::GridTemplate>>,
    /// `grid-template-areas` (CSS Grid 2 §7.3): the explicit grid's named
    /// areas.
    pub grid_template_areas: Option<Value<crate::layout::GridTemplateAreas>>,
    /// `grid-auto-columns` (CSS Grid 2 §7.6): the implicit columns'
    /// sizes, a pattern of one or more.
    pub grid_auto_columns: Option<Value<Vec<crate::layout::TrackSize>>>,
    /// `grid-auto-rows` (§7.6): the implicit rows' sizes.
    pub grid_auto_rows: Option<Value<Vec<crate::layout::TrackSize>>>,
    /// `grid-auto-flow` (§7.7): how auto-placement fills the grid.
    pub grid_auto_flow: Option<Value<crate::layout::GridAutoFlow>>,
    /// `grid-row-start` / `-end`, `grid-column-start` / `-end` (§8.3):
    /// the item's grid area; set together by `grid-row`, `grid-column`
    /// and `grid-area` (§8.4).
    pub grid_row_start: Option<Value<crate::layout::GridLine>>,
    pub grid_row_end: Option<Value<crate::layout::GridLine>>,
    pub grid_column_start: Option<Value<crate::layout::GridLine>>,
    pub grid_column_end: Option<Value<crate::layout::GridLine>>,
}
