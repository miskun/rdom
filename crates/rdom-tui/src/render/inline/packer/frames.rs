//! The inline boxes the packer is inside, for the line box heights
//! (CSS 2.1 §10.8.1): each inline box is as tall as its `line-height`,
//! its glyph row on the line's baseline row with half the leading above
//! and half below; a line box spans every inline box with content on it
//! and the block's strut ("a zero-width inline box with the element's
//! font and line height properties"), which is on every line.
//!
//! Frames live in an arena for the whole inline formatting context; a
//! grapheme or atom names the frame it is in, and placing it on a line
//! marks that frame and its ancestors (an inline box with content on a
//! line is on that line). Settling a line reads the marked frames'
//! extents and starts the next line with only the strut marked.

use rdom_style::layout::LineHeight;

/// Rows of an inline box above and below its baseline row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::render::inline) struct BoxRows {
    pub(in crate::render::inline) above: u16,
    pub(in crate::render::inline) below: u16,
}

impl BoxRows {
    /// An inline box of `line_height` (CSS 2.1 §10.8.1's half-leading,
    /// [`LineHeight::half_leading`]).
    pub(in crate::render::inline) fn of(line_height: &LineHeight) -> Self {
        let (above, below) = line_height.half_leading();
        BoxRows { above, below }
    }

    /// The rows holding both.
    pub(in crate::render::inline) fn max(self, other: BoxRows) -> BoxRows {
        BoxRows {
            above: self.above.max(other.above),
            below: self.below.max(other.below),
        }
    }
}

/// Index of a frame in [`Frames`]; the strut is `0`.
pub(super) type FrameId = u32;

/// One inline box.
#[derive(Debug, Clone, Copy)]
struct Frame {
    /// The box it is in; the strut is its own parent.
    parent: FrameId,
    rows: BoxRows,
    /// The line it was last marked on, plus one (0: never).
    marked: usize,
}

/// The inline boxes of one inline formatting context.
#[derive(Debug, Clone)]
pub(super) struct Frames {
    frames: Vec<Frame>,
    /// The box content is being taken into.
    current: FrameId,
    /// The current line, plus one (the `marked` value it sets).
    line: usize,
    /// The extent of the frames marked on the current line.
    extent: BoxRows,
}

impl Frames {
    /// The frames of a context whose block has the strut `strut`.
    pub(super) fn new(strut: BoxRows) -> Self {
        Frames {
            frames: vec![Frame {
                parent: 0,
                rows: strut,
                marked: 1,
            }],
            current: 0,
            line: 1,
            extent: strut,
        }
    }

    /// The block's strut.
    pub(super) fn strut(&self) -> BoxRows {
        self.frames[0].rows
    }

    /// Content now goes into a new inline box `rows` tall, inside the
    /// current one.
    pub(super) fn enter(&mut self, rows: BoxRows) {
        let id = self.frames.len() as FrameId;
        self.frames.push(Frame {
            parent: self.current,
            rows,
            marked: 0,
        });
        self.current = id;
    }

    /// The current inline box ends; content goes into its parent.
    pub(super) fn leave(&mut self) {
        self.current = self.frames[self.current as usize].parent;
    }

    /// The box content is being taken into.
    pub(super) fn current(&self) -> FrameId {
        self.current
    }

    /// Content of `frame` is placed on the current line: it and its
    /// ancestors are on the line.
    pub(super) fn mark(&mut self, frame: FrameId) {
        let mut id = frame as usize;
        while let Some(f) = self.frames.get_mut(id)
            && f.marked != self.line
        {
            f.marked = self.line;
            self.extent = self.extent.max(f.rows);
            id = f.parent as usize;
        }
    }

    /// The current line's extent around its baseline row; the next line
    /// starts with the strut alone.
    pub(super) fn settle(&mut self) -> BoxRows {
        let extent = self.extent;
        self.line += 1;
        self.frames[0].marked = self.line;
        self.extent = self.frames[0].rows;
        extent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(above: u16, below: u16) -> BoxRows {
        BoxRows { above, below }
    }

    /// CSS 2.1 §10.8.1: the strut is on every line; an inline box counts
    /// on the lines its content is placed on, with its ancestors.
    #[test]
    fn a_line_spans_the_strut_and_the_boxes_on_it() {
        let mut f = Frames::new(rows(0, 1));
        f.enter(rows(2, 0));
        f.enter(rows(0, 0));
        let inner = f.current();
        f.leave();
        f.leave();
        assert_eq!(f.settle(), rows(0, 1), "nothing placed: the strut");
        f.mark(inner);
        assert_eq!(f.settle(), rows(2, 1), "the inner box marks its parent");
        assert_eq!(f.settle(), rows(0, 1), "the next line starts afresh");
    }
}
