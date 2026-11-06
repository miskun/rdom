//! The frame a session shows: its grid and hash, built once per step and
//! reused by every capture of it, and what changed from one to the next.

use rdom_tui::render::TestBackend;
use rdom_tui::{App, Buffer, Cell};

use crate::capture::{Grid, grid_hash};

/// The current frame's grid and hash.
pub(super) struct Frame {
    pub(super) grid: Grid,
    pub(super) hash: String,
}

impl Frame {
    /// The frame `app` shows, at `depth`.
    pub(super) fn of(app: &App<TestBackend>) -> Frame {
        let grid = Grid::from_buffer(app.terminal().buffer(), app.color_depth());
        let hash = grid_hash(&grid);
        Frame { grid, hash }
    }
}

/// How many cells differ between `a` and `b`, and the rectangle holding
/// them. Buffers of different sizes differ everywhere `b` covers.
pub(super) fn diff(a: &Buffer, b: &Buffer) -> (u32, Option<[u16; 4]>) {
    let area = b.area;
    let (mut n, mut min_x, mut min_y, mut max_x, mut max_y) = (0u32, u16::MAX, u16::MAX, 0, 0);
    let same_size = a.area == b.area;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if same_size && same_look(a.cell(x, y), b.cell(x, y)) {
                continue;
            }
            n += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    let bounds = (n > 0).then(|| [min_x, min_y, max_x - min_x + 1, max_y - min_y + 1]);
    (n, bounds)
}

/// Whether two cells show the same thing, as the grid records it: glyph
/// (a blank cell and a space are the same), spacer, colors, attributes,
/// underline color and link — not the renderer's diff hint.
fn same_look(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.symbol() == b.symbol()
                && a.is_spacer() == b.is_spacer()
                && a.fg == b.fg
                && a.bg == b.bg
                && a.modifier == b.modifier
                && a.underline_color == b.underline_color
                && a.link == b.link
        }
        (a, b) => a.is_none() && b.is_none(),
    }
}
