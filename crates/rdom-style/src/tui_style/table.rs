//! [`TableDeclarations`]: a style block's declarations of the table
//! properties, the specified side of [`TableStyle`](crate::layout::TableStyle).

use crate::Value;
use crate::layout::{CaptionSide, EmptyCells, TableLayout};

/// The table properties a [`TuiStyle`](crate::TuiStyle) declares
/// ([`TuiStyle::table`](crate::TuiStyle::table)), one field per longhand,
/// `None` where the block does not declare it.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableDeclarations {
    /// `table-layout` (CSS 2.1 §17.5.2).
    pub table_layout: Option<Value<TableLayout>>,
    /// `caption-side` (§17.4.1).
    pub caption_side: Option<Value<CaptionSide>>,
    /// `empty-cells` (§17.6.1.1).
    pub empty_cells: Option<Value<EmptyCells>>,
}
