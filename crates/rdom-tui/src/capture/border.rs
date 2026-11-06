//! A cell's border state as a paint trace reports it: per direction, the
//! line that won it — its style, weight and color — and whether a `hidden`
//! border suppressed it; and the half-block quadrants. The joiner draws the
//! glyph from this ([`OpKind::Join`](super::OpKind::Join)); the
//! conflict-resolution bookkeeping behind it stays the renderer's.

use rdom_style::layout::{BorderStyle, BorderWeight};

use crate::render::buffer::{BorderCell, BorderDirState};
use crate::style::Color;

/// A cell's border state, directions in the order north, east, south,
/// west.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BorderState {
    /// The line that won each direction, if any.
    pub lines: [Option<BorderLine>; 4],
    /// A `hidden` border suppresses each direction (CSS Tables 3 §11.5).
    pub hidden: [bool; 4],
    /// The half-block quadrants half-block borders fill, a 4-bit set
    /// (top-left, top-right, bottom-left, bottom-right).
    pub quadrants: u8,
}

/// The line that won one direction of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BorderLine {
    pub style: BorderStyle,
    pub weight: BorderWeight,
    pub color: Color,
}

impl BorderState {
    pub(crate) fn of(cell: &BorderCell, quadrants: u8) -> Self {
        let line = |d: &BorderDirState| {
            d.winner.map(|w| BorderLine {
                style: w.style,
                weight: w.weight,
                color: w.fg,
            })
        };
        BorderState {
            lines: [
                line(&cell[0]),
                line(&cell[1]),
                line(&cell[2]),
                line(&cell[3]),
            ],
            hidden: [
                cell[0].killed,
                cell[1].killed,
                cell[2].killed,
                cell[3].killed,
            ],
            quadrants,
        }
    }
}
