//! The table values (CSS 2.1 §17, CSS Tables 3): the layout-internal
//! `display` types a table is built from ([`TablePart`]), `table-layout`,
//! `caption-side` — and [`TableStyle`], the computed group of the table
//! properties. (`border-collapse` and `border-spacing` are older, and
//! stay top-level fields of the style records.)

/// A layout-internal display type of the table model (CSS Display 3 §2.4,
/// CSS 2.1 §17.2): what [`Display::TablePart`](super::Display::TablePart)
/// holds. Each is a box that only makes sense inside a table: a stray one
/// is wrapped in the anonymous table boxes it needs (CSS 2.1 §17.2.1),
/// and a flex or grid item or a float is blockified to `block` (CSS
/// Display 3 §2.7, CSS 2.1 §9.7).
///
/// Closed (DESIGN): every part is a box the table layout places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TablePart {
    /// `table-row-group` (`<tbody>`).
    RowGroup,
    /// `table-header-group` (`<thead>`): the first one's rows come first.
    HeaderGroup,
    /// `table-footer-group` (`<tfoot>`): the first one's rows come last.
    FooterGroup,
    /// `table-row` (`<tr>`).
    Row,
    /// `table-cell` (`<td>`, `<th>`): a block container inside
    /// (`flow-root`).
    Cell,
    /// `table-column-group` (`<colgroup>`).
    ColumnGroup,
    /// `table-column` (`<col>`).
    Column,
    /// `table-caption` (`<caption>`): a block container inside
    /// (`flow-root`), placed above or below the table (`caption-side`).
    Caption,
}

impl TablePart {
    /// Every part with its `display` keyword.
    pub const KEYWORDS: &'static [(&'static str, TablePart)] = &[
        ("table-row-group", TablePart::RowGroup),
        ("table-header-group", TablePart::HeaderGroup),
        ("table-footer-group", TablePart::FooterGroup),
        ("table-row", TablePart::Row),
        ("table-cell", TablePart::Cell),
        ("table-column-group", TablePart::ColumnGroup),
        ("table-column", TablePart::Column),
        ("table-caption", TablePart::Caption),
    ];

    /// The `display` keyword.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, p)| *p == self)
            .map_or("table-cell", |(k, _)| k)
    }

    /// Whether the part lays its own content out, as a block container
    /// (`flow-root`): a cell or a caption.
    pub fn is_block_container(self) -> bool {
        matches!(self, TablePart::Cell | TablePart::Caption)
    }

    /// Whether the part is a row group: `table-row-group`,
    /// `table-header-group` or `table-footer-group`.
    pub fn is_row_group(self) -> bool {
        matches!(
            self,
            TablePart::RowGroup | TablePart::HeaderGroup | TablePart::FooterGroup
        )
    }
}

/// `table-layout` (CSS 2.1 §17.5.2): how a table's columns are sized. Not
/// inherited; initial `auto`.
///
/// Closed (DESIGN): the two algorithms the table layout runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TableLayout {
    /// The automatic algorithm (§17.5.2.2): every cell's content sizes
    /// the columns.
    #[default]
    Auto,
    /// The fixed algorithm (§17.5.2.1): the table's width, the columns'
    /// widths and the first row's cells size the columns — when the
    /// table's width is not `auto`.
    Fixed,
}

impl TableLayout {
    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        match self {
            TableLayout::Auto => "auto",
            TableLayout::Fixed => "fixed",
        }
    }
}

/// `caption-side` (CSS 2.1 §17.4.1): which side of the table its captions
/// go — in CSS Tables 3 the table's block-start or block-end side, so
/// `block-start` / `block-end` parse as `top` / `bottom`. Inherited;
/// initial `top`.
///
/// Closed (DESIGN): a side the table layout places captions on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CaptionSide {
    /// Above the table (block-start).
    #[default]
    Top,
    /// Below the table (block-end).
    Bottom,
}

impl CaptionSide {
    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        match self {
            CaptionSide::Top => "top",
            CaptionSide::Bottom => "bottom",
        }
    }
}

/// The computed table properties of an element
/// ([`ComputedStyle::table`](crate::ComputedStyle::table)).
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TableStyle {
    /// `table-layout` (§17.5.2). Not inherited.
    pub table_layout: TableLayout,
    /// `caption-side` (§17.4.1). Inherited.
    pub caption_side: CaptionSide,
}
