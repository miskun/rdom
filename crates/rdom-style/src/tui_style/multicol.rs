//! [`MulticolDeclarations`] and [`FragmentationDeclarations`]: a style
//! block's declarations of the multi-column and fragmentation properties,
//! the specified side of [`MulticolStyle`](crate::layout::MulticolStyle)
//! and [`FragmentationStyle`](crate::layout::FragmentationStyle).

use crate::layout::{
    BorderStyle, BorderWidth, BoxDecorationBreak, BreakBetween, BreakInside, ColumnCount,
    ColumnFill, ColumnSpan, ColumnWidth,
};
use crate::{TuiColor, Value};

/// The CSS Multi-column 1 properties a [`TuiStyle`](crate::TuiStyle)
/// declares ([`TuiStyle::multicol`](crate::TuiStyle::multicol)), one
/// field per longhand, `None` where the block does not declare it.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MulticolDeclarations {
    /// `column-count` (§3.2).
    pub column_count: Option<Value<ColumnCount>>,
    /// `column-width` (§3.1).
    pub column_width: Option<Value<ColumnWidth>>,
    /// `column-rule-style` (§4.2).
    pub column_rule_style: Option<Value<BorderStyle>>,
    /// `column-rule-width` (§4.3).
    pub column_rule_width: Option<Value<BorderWidth>>,
    /// `column-rule-color` (§4.1).
    pub column_rule_color: Option<Value<TuiColor>>,
    /// `column-span` (§6.1).
    pub column_span: Option<Value<ColumnSpan>>,
    /// `column-fill` (§7.1).
    pub column_fill: Option<Value<ColumnFill>>,
}

/// The CSS Fragmentation 3 properties a [`TuiStyle`](crate::TuiStyle)
/// declares ([`TuiStyle::fragmentation`](crate::TuiStyle::fragmentation)).
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FragmentationDeclarations {
    /// `break-before` (§3.1); the legacy `page-break-before` writes it.
    pub break_before: Option<Value<BreakBetween>>,
    /// `break-after` (§3.1).
    pub break_after: Option<Value<BreakBetween>>,
    /// `break-inside` (§3.2).
    pub break_inside: Option<Value<BreakInside>>,
    /// `orphans` (§3.3).
    pub orphans: Option<Value<u32>>,
    /// `widows` (§3.3).
    pub widows: Option<Value<u32>>,
    /// `box-decoration-break` (§5.4).
    pub box_decoration_break: Option<Value<BoxDecorationBreak>>,
}
