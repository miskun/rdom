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
    ///
    /// The repaint is compared with the frame: [`PaintTrace::frame_mismatch`]
    /// says how many cells differ (0 unless it saw other state).
    pub fn trace_paint(&self, region: Rect) -> PaintTrace {
        let frame = self.terminal.buffer();
        let (buf, trace) = trace_paint(&self.dom, frame.area, region);
        let differ = frame
            .content
            .iter()
            .zip(&buf.content)
            .filter(|(a, b)| a != b)
            .count();
        trace.with_frame_mismatch(differ)
    }
}
