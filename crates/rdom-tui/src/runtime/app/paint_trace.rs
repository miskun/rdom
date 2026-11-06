//! The frame, traced (`crate::capture`): [`App::trace_paint`].

use super::App;
use crate::capture::{PaintTrace, trace_paint};
use crate::render::{Backend, Rect};

impl<B: Backend> App<B> {
    /// Paint the last drawn frame again with a recorder on the cells of
    /// `region`, and return what painted each of them
    /// ([`crate::capture`]). The document is not changed; the repaint
    /// reads the cascade and layout the frame used, so its cells are the
    /// frame's — call it after [`draw_if_dirty`](Self::draw_if_dirty) or
    /// [`advance`](Self::advance), not between a change and its frame.
    pub fn trace_paint(&self, region: Rect) -> PaintTrace {
        let area = self.terminal.buffer().area;
        trace_paint(&self.dom, area, region).1
    }
}
