//! A laid-out grid's lines, kept on its container (`TuiExt::kept`)
//! for the absolutely positioned boxes whose containing block it is
//! (CSS Grid 2 §9.1): "the containing block corresponds to the grid area
//! determined by its grid-placement properties", an `auto` line — or one
//! the grid does not have — being the containing block's own edge.

use std::borrow::Cow;

use rdom_core::{Dom, NodeId};

use super::placement::{Edge, Lines, count};
use crate::ext::TuiExt;
use crate::layout::{GridLine, LayoutRect};
use crate::style::ComputedStyle;

/// A grid's two axes of lines.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GridLines {
    pub(super) columns: AxisLines,
    pub(super) rows: AxisLines,
    /// The grid's columns run from the right (`direction: rtl`).
    pub(super) rtl: bool,
    /// Its items that subgrid an axis, and their grid areas' spans — for
    /// their own layout, which takes its tracks from these lines (§9).
    pub(super) subgrids: Vec<(NodeId, super::track::Span, super::track::Span)>,
}

/// One axis of a laid-out grid.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct AxisLines {
    /// The explicit grid's tracks and its lines' names.
    pub(super) explicit: usize,
    pub(super) names: Vec<Vec<String>>,
    /// The implicit tracks before the explicit grid.
    pub(super) before: usize,
    /// Each track's start-side and end-side edge, in cells from the
    /// container's content-box start edge on this axis (the right one of
    /// an `rtl` grid's columns), unscrolled. Relative, not absolute, so a
    /// grid moved after its layout (`tree::shift_subtree`) keeps them
    /// true with no copy of its own: the container's `content_layout`
    /// is the one origin they resolve against (C7G-LINES-SHIFT).
    pub(super) edges: Vec<(i32, i32)>,
}

impl GridLines {
    /// Each column's and row's cells as a range from the content box's
    /// left / top edge, in grid order (`TuiAccessors::grid_tracks`, CSS
    /// Grid 2 §7.2.6): the edges are offsets from the content-box start
    /// on each axis, which for an `rtl` grid's columns is the right edge
    /// of a box `content_width` wide.
    pub(crate) fn used_tracks(
        &self,
        content_width: u16,
    ) -> (Vec<std::ops::Range<i32>>, Vec<std::ops::Range<i32>>) {
        let w = i32::from(content_width);
        let rtl = self.rtl;
        let columns = self
            .columns
            .edges
            .iter()
            .map(|&(s, e)| if rtl { w - e..w - s } else { s..e })
            .collect();
        let rows = self.rows.edges.iter().map(|&(s, e)| s..e).collect();
        (columns, rows)
    }
}

impl AxisLines {
    /// The two edges of the area `start` / `end` name on this axis, `cb`
    /// (start side, end side) standing for an `auto` or missing line;
    /// `at` places a track edge (an offset from the content-box start).
    fn area(
        &self,
        start: &GridLine,
        end: &GridLine,
        cb: (i32, i32),
        at: impl Fn(i32) -> i32,
    ) -> (i32, i32) {
        let names: Vec<Vec<Cow<'_, str>>> = self
            .names
            .iter()
            .map(|n| n.iter().map(|s| Cow::Borrowed(s.as_str())).collect())
            .collect();
        let lines = Lines {
            tracks: self.explicit,
            names: &names,
        };
        let s = lines.definite(start, Edge::Start);
        let e = lines.definite(end, Edge::End);
        // A span reaches from a definite other edge; beside `auto` it is
        // `auto` itself.
        let span = |line: &GridLine, from: i32, forwards: bool| match line {
            GridLine::Span {
                count: n,
                name: Some(name),
            } => Some(lines.search(from, name, count(*n), forwards)),
            GridLine::Span { count: n, .. } if forwards => Some(from + count(*n)),
            GridLine::Span { count: n, .. } => Some(from - count(*n)),
            _ => None,
        };
        let (s, e) = match (s, e) {
            (Some(s), Some(e)) if s == e => (Some(s), None),
            (Some(s), Some(e)) => (Some(s.min(e)), Some(s.max(e))),
            (Some(s), None) => (Some(s), span(end, s, true)),
            (None, Some(e)) => (span(start, e, false), Some(e)),
            (None, None) => (None, None),
        };
        // An explicit-grid line number as a line of the implicit grid,
        // if it has it.
        let line = |x: i32| {
            usize::try_from(x + self.before as i32)
                .ok()
                .filter(|&i| i <= self.edges.len() && !self.edges.is_empty())
        };
        let start_edge = s.and_then(line).map_or(cb.0, |i| {
            at(match self.edges.get(i) {
                Some(&(start, _)) => start,
                None => self.edges[i - 1].1,
            })
        });
        let end_edge = e.and_then(line).map_or(cb.1, |i| {
            at(match i {
                0 => self.edges[0].0,
                i => self.edges[i - 1].1,
            })
        });
        (start_edge, end_edge)
    }
}

/// The containing block an absolutely positioned box styled `c` — an
/// element or a `::before` / `::after` — gets from the grid container
/// `grid` (§9.1), whose box rdom takes as a containing block is `cb`: its
/// grid area, an `auto` or missing line at `cb`'s edge. `None` when
/// `grid` is not a laid-out grid container.
pub(crate) fn abspos_area(
    dom: &Dom<TuiExt>,
    c: &ComputedStyle,
    grid: NodeId,
    cb: LayoutRect,
) -> Option<LayoutRect> {
    let ext = dom.node(grid).ext()?;
    let lines = ext.grid_lines()?;
    // The lines count from the content box the grid has now — moved
    // with it, if it moved after its layout.
    let content = ext.content_layout;
    let rtl = lines.rtl;
    let (right, bottom) = (cb.x + i32::from(cb.width), cb.y + i32::from(cb.height));
    let x_cb = if rtl { (right, cb.x) } else { (cb.x, right) };
    let content_right = content.x + i32::from(content.width);
    let (x0, x1) = lines.columns.area(
        &c.grid.grid_column_start,
        &c.grid.grid_column_end,
        x_cb,
        |x| {
            if rtl {
                content_right - x
            } else {
                content.x + x
            }
        },
    );
    let (y0, y1) = lines.rows.area(
        &c.grid.grid_row_start,
        &c.grid.grid_row_end,
        (cb.y, bottom),
        |y| content.y + y,
    );
    let extent = |a: i32, b: i32| (a - b).unsigned_abs().min(u32::from(u16::MAX)) as u16;
    Some(LayoutRect::new(
        x0.min(x1),
        y0.min(y1),
        extent(x0, x1),
        extent(y0, y1),
    ))
}
