//! What a formatting context keeps of its last layout for the readers
//! after it: a grid container's lines (CSS Grid 2 §9.1), a table's table
//! box inside its wrapper (CSS 2.1 §17.4, C13G-TABLE-GEOMETRY) and its
//! tracks (`TuiAccessors::table_tracks`, C13G-TABLE-TRACKS), a
//! multi-column container's column boxes and a fragmented box's fragments
//! (C15-COLUMNS). One boxed record on [`TuiExt`] — a box is one of them
//! at most (a grid, a table and a multi-column container are monolithic,
//! never fragmented) — so the common box pays one pointer and nothing
//! more.

use std::ops::Range;

use super::TuiExt;
use crate::layout::LayoutRect;
use crate::render::layout_pass::GridLines;
pub(crate) use crate::render::layout_pass::fragment::BoxFragments;

/// A box's kept layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeptLayout {
    /// A grid container's lines.
    Grid(GridLines),
    /// A table's table box in its wrapper box, and its tracks.
    Table(TableKept),
    /// A multi-column container's column boxes (CSS Multi-column 1 §2),
    /// one set per run of columns between spanners.
    Columns(Vec<ColumnSet>),
    /// A box split across fragmentainers (CSS Fragmentation 3 §5.4): its
    /// fragments.
    Fragments(BoxFragments),
}

/// A row of a multi-column container's column boxes: its rows, from the
/// content box's top edge, and its columns. Relative, so a subtree shift
/// keeps it true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnSet {
    pub(crate) top: i32,
    pub(crate) height: u16,
    pub(crate) columns: Vec<ColumnBox>,
}

/// One column box: its left edge from the content box's left edge, its
/// width, and whether content fell in it (a rule is drawn only between two
/// that hold some, CSS Multi-column 1 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnBox {
    pub(crate) x: i32,
    pub(crate) width: u16,
    pub(crate) filled: bool,
}

/// What a laid-out table keeps: where its table box sits in its wrapper,
/// and its columns and rows — each a cell range from the table box's
/// content edge, unscrolled, in column / row order (the shape
/// [`TableTracks`](crate::TableTracks) reports). Relative, as the
/// insets, so a subtree shift keeps them true.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TableKept {
    pub(crate) insets: TableInsets,
    pub(crate) columns: Vec<Range<i32>>,
    pub(crate) rows: Vec<Range<i32>>,
}

/// Where a table's table box sits in its wrapper box (`TuiExt::layout`,
/// captions included, CSS 2.1 §17.4): the rows its captions take above
/// and below it. Relative to the wrapper, so moving the box (a subtree
/// shift) moves it too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TableInsets {
    pub(crate) above: u16,
    pub(crate) below: u16,
}

impl TuiExt {
    /// A grid container's lines after its last layout; `None` for any
    /// other box.
    pub(crate) fn grid_lines(&self) -> Option<&GridLines> {
        match self.kept.as_deref()? {
            KeptLayout::Grid(lines) => Some(lines),
            _ => None,
        }
    }

    /// A table's kept layout after its last layout; `None` for any other
    /// box.
    pub(crate) fn table_kept(&self) -> Option<&TableKept> {
        match self.kept.as_deref()? {
            KeptLayout::Table(t) => Some(t),
            _ => None,
        }
    }

    /// A multi-column container's column sets after its last layout;
    /// none for any other box.
    pub(crate) fn column_sets(&self) -> &[ColumnSet] {
        match self.kept.as_deref() {
            Some(KeptLayout::Columns(sets)) => sets,
            _ => &[],
        }
    }

    /// The fragments of a box its last layout split across fragmentainers
    /// (CSS Fragmentation 3); `None` for a box in one piece.
    pub(crate) fn box_fragments(&self) -> Option<&BoxFragments> {
        match self.kept.as_deref()? {
            KeptLayout::Fragments(f) => Some(f),
            _ => None,
        }
    }

    /// Whether this is a table with captions: its wrapper box is taller
    /// than its table box (CSS 2.1 §17.4).
    pub(crate) fn has_captions(&self) -> bool {
        self.table_kept()
            .is_some_and(|t| t.insets.above > 0 || t.insets.below > 0)
    }

    /// The box this element's border, background, scrollport, overflow
    /// clip and resizer are on — one answer for paint, the scroll
    /// machinery, clipping and hit-testing: a table's table box (CSS 2.1
    /// §17.4: `overflow` and the border apply to it, not to the wrapper
    /// around it and its captions), every other box's border box
    /// (`layout`).
    pub(crate) fn border_box(&self) -> LayoutRect {
        match self.kept.as_deref() {
            Some(KeptLayout::Table(TableKept { insets: t, .. })) => {
                let l = self.layout;
                LayoutRect::new(
                    l.x,
                    l.y + i32::from(t.above),
                    l.width,
                    l.height.saturating_sub(t.above.saturating_add(t.below)),
                )
            }
            _ => self.layout,
        }
    }
}
