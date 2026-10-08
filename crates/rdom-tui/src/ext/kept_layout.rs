//! What a formatting context keeps of its last layout for the readers
//! after it: a grid container's lines (CSS Grid 2 §9.1), a table's table
//! box inside its wrapper (CSS 2.1 §17.4, C13G-TABLE-GEOMETRY). One boxed
//! record on [`TuiExt`] — a box is a grid or a table, never both — so the
//! common box pays one pointer for either and nothing more.

use super::TuiExt;
use crate::layout::LayoutRect;
use crate::render::layout_pass::GridLines;

/// A box's kept layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeptLayout {
    /// A grid container's lines.
    Grid(GridLines),
    /// A table's table box in its wrapper box.
    Table(TableInsets),
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
            KeptLayout::Table(_) => None,
        }
    }

    /// The box this element's border, background, scrollport, overflow
    /// clip and resizer are on — one answer for paint, the scroll
    /// machinery, clipping and hit-testing: a table's table box (CSS 2.1
    /// §17.4: `overflow` and the border apply to it, not to the wrapper
    /// around it and its captions), every other box's border box
    /// (`layout`).
    pub(crate) fn border_box(&self) -> LayoutRect {
        match self.kept.as_deref() {
            Some(KeptLayout::Table(t)) => {
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
