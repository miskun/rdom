//! The inline boxes the packer is inside, for the line box heights and
//! the rows of their content (CSS 2.1 §10.8, §10.8.1): each inline box —
//! an inline element's, generated text's, an atomic inline's — is as tall
//! as its `line-height` (an atom: its margin box), and sits in its line by
//! its `vertical-align`: raised or lowered from its parent's baseline, or
//! as the root of an aligned subtree at the line box's top or bottom. A
//! line box spans the boxes with content on it and the block's strut
//! ("a zero-width inline box with the element's font and line height
//! properties"), which is on every line.
//!
//! Frames live in an arena for the whole inline formatting context; a
//! grapheme or atom names the frame it is in, and placing it on a line
//! marks that frame and its ancestors (an inline box with content on a
//! line is on that line). Settling a line reads the marked frames'
//! extents, per aligned subtree, and starts the next line with only the
//! strut marked.

use rdom_style::layout::{LineHeight, VerticalAlign};

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
}

/// How an inline box is aligned in its line (`vertical-align`, CSS 2.1
/// §10.8.1), resolved against its own rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::render::inline) enum BoxAlign {
    /// Raised this many rows above its parent's baseline (negative:
    /// lowered).
    Shift(i32),
    /// Its middle row on its parent's baseline row.
    Middle,
    /// Its top on its parent's glyph row (the top of its content area).
    TextTop,
    /// Its bottom on its parent's glyph row.
    TextBottom,
    /// Its aligned subtree at the line box's top.
    Top,
    /// Its aligned subtree at the line box's bottom.
    Bottom,
    /// On its parent's baseline.
    #[default]
    Baseline,
}

impl BoxAlign {
    /// The alignment `vertical-align: va` asks for. `sub` / `super` move
    /// the box one row — the parent's subscript / superscript position on
    /// a grid (DIVERGENCES §2).
    pub(in crate::render::inline) fn of(va: &VerticalAlign) -> Self {
        match va {
            VerticalAlign::Baseline => BoxAlign::Baseline,
            VerticalAlign::Sub => BoxAlign::Shift(-1),
            VerticalAlign::Super => BoxAlign::Shift(1),
            VerticalAlign::TextTop => BoxAlign::TextTop,
            VerticalAlign::TextBottom => BoxAlign::TextBottom,
            VerticalAlign::Middle => BoxAlign::Middle,
            VerticalAlign::Top => BoxAlign::Top,
            VerticalAlign::Bottom => BoxAlign::Bottom,
            length => BoxAlign::Shift(length.raise().unwrap_or(0)),
        }
    }

    /// Rows a box `rows` tall is raised above its parent's baseline; `None`
    /// for `top` / `bottom`, which align it with the line box instead.
    fn raise(self, rows: BoxRows) -> Option<i32> {
        let (above, below) = (i32::from(rows.above), i32::from(rows.below));
        Some(match self {
            BoxAlign::Baseline => 0,
            BoxAlign::Shift(n) => n,
            // The middle row of a box `above + 1 + below` rows tall — the
            // upper one of two, as `center` rounds its leading space down.
            BoxAlign::Middle => (above + below) / 2 - above,
            BoxAlign::TextTop => -above,
            BoxAlign::TextBottom => below,
            BoxAlign::Top | BoxAlign::Bottom => return None,
        })
    }
}

/// Index of a frame in [`Frames`]; the strut is `0`.
pub(in crate::render::inline) type FrameId = u32;

/// One inline box.
#[derive(Debug, Clone, Copy)]
struct Frame {
    /// The box it is in; the strut is its own parent.
    parent: FrameId,
    rows: BoxRows,
    /// The root of its aligned subtree: the strut, or the nearest `top` /
    /// `bottom` box among it and its ancestors.
    root: FrameId,
    /// Its baseline's rows above its subtree root's.
    raise: i32,
    /// `Top` or `Bottom` on a subtree root, else `Baseline`.
    align: BoxAlign,
    /// The line it was last marked on, plus one (0: never).
    marked: usize,
}

/// One aligned subtree's extent on the current line: rows above and
/// below its root's baseline row.
#[derive(Debug, Clone, Copy)]
struct Extent {
    root: FrameId,
    above: i32,
    below: i32,
}

/// The rows of a settled line: its height, the strut's baseline row from
/// the line's top, and each other aligned subtree's — kept only when the
/// line has one, so a line of plain content allocates nothing here.
#[derive(Debug, Clone, Default)]
pub(super) struct Settled {
    pub(super) height: u16,
    strut: u16,
    others: Vec<(FrameId, u16)>,
}

impl Settled {
    /// The line's baseline row: the strut's.
    pub(super) fn baseline(&self) -> u16 {
        self.strut
    }
}

/// The inline boxes of one inline formatting context.
#[derive(Debug, Clone)]
pub(super) struct Frames {
    frames: Vec<Frame>,
    /// The box content is being taken into.
    current: FrameId,
    /// The current line, plus one (the `marked` value it sets).
    line: usize,
    /// The extents of the aligned subtrees with content on the current
    /// line, the strut's first.
    extents: Vec<Extent>,
}

impl Frames {
    /// The frames of a context whose block has the strut `strut`.
    pub(super) fn new(strut: BoxRows) -> Self {
        Frames {
            frames: vec![Frame {
                parent: 0,
                rows: strut,
                root: 0,
                raise: 0,
                align: BoxAlign::Baseline,
                marked: 1,
            }],
            current: 0,
            line: 1,
            extents: vec![Self::strut_extent(strut)],
        }
    }

    fn strut_extent(strut: BoxRows) -> Extent {
        Extent {
            root: 0,
            above: i32::from(strut.above),
            below: i32::from(strut.below),
        }
    }

    /// The block's strut.
    pub(super) fn strut(&self) -> BoxRows {
        self.frames[0].rows
    }

    /// Content now goes into a new inline box `rows` tall aligned by
    /// `align`, inside the current one.
    pub(super) fn enter(&mut self, rows: BoxRows, align: BoxAlign) {
        let id = self.add(rows, align);
        self.current = id;
    }

    /// A new box `rows` tall aligned by `align`, inside the current one,
    /// that takes no content of its own (an atomic inline).
    pub(super) fn add(&mut self, rows: BoxRows, align: BoxAlign) -> FrameId {
        let id = self.frames.len() as FrameId;
        let parent = self.frames[self.current as usize];
        let (root, raise, align) = match align.raise(rows) {
            Some(raise) => (parent.root, parent.raise + raise, BoxAlign::Baseline),
            None => (id, 0, align),
        };
        self.frames.push(Frame {
            parent: self.current,
            rows,
            root,
            raise,
            align,
            marked: 0,
        });
        id
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
            let f = *f;
            let above = f.raise + i32::from(f.rows.above);
            let below = i32::from(f.rows.below) - f.raise;
            match self.extents.iter_mut().find(|e| e.root == f.root) {
                Some(e) => {
                    e.above = e.above.max(above);
                    e.below = e.below.max(below);
                }
                None => self.extents.push(Extent {
                    root: f.root,
                    above,
                    below,
                }),
            }
            id = f.parent as usize;
        }
    }

    /// Settle the current line (CSS 2.1 §10.8): the strut's subtree sets
    /// its baseline and height; a `top` / `bottom` subtree taller than
    /// that grows it away from the edge it is aligned with. The next line
    /// starts with the strut alone.
    pub(super) fn settle(&mut self) -> Settled {
        self.line += 1;
        self.frames[0].marked = self.line;
        let row = |v: i32| v.clamp(0, i32::from(u16::MAX)) as u16;
        let extents = &self.extents;
        let (mut above, mut below) = (extents[0].above, extents[0].below);
        for e in &extents[1..] {
            let tall = (e.above + 1 + e.below) - (above + 1 + below);
            if tall > 0 {
                match self.frames[e.root as usize].align {
                    BoxAlign::Bottom => above += tall,
                    _ => below += tall,
                }
            }
        }
        let height = above + 1 + below;
        let others = extents[1..]
            .iter()
            .map(|e| {
                let baseline = match self.frames[e.root as usize].align {
                    BoxAlign::Top => e.above,
                    BoxAlign::Bottom => height - 1 - e.below,
                    _ => above,
                };
                (e.root, row(baseline))
            })
            .collect();
        // The next line starts with the strut's extent alone, in the same
        // buffer.
        self.extents.truncate(1);
        self.extents[0] = Self::strut_extent(self.frames[0].rows);
        Settled {
            height: row(height),
            strut: row(above),
            others,
        }
    }

    /// The row `frame`'s baseline sits on in the settled line `settled`.
    pub(super) fn row(&self, settled: &Settled, frame: FrameId) -> u16 {
        let f = &self.frames[frame as usize];
        let baseline = settled
            .others
            .iter()
            .find(|&&(root, _)| root == f.root)
            .map_or(settled.strut, |&(_, row)| row);
        (i32::from(baseline) - f.raise).clamp(0, i32::from(u16::MAX)) as u16
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
        f.enter(rows(2, 0), BoxAlign::Baseline);
        f.enter(rows(0, 0), BoxAlign::Baseline);
        let inner = f.current();
        f.leave();
        f.leave();
        let s = f.settle();
        assert_eq!(
            (s.baseline(), s.height),
            (0, 2),
            "nothing placed: the strut"
        );
        f.mark(inner);
        let s = f.settle();
        assert_eq!(
            (s.baseline(), s.height),
            (2, 4),
            "the inner box marks its parent"
        );
        assert_eq!(f.settle().height, 2, "the next line starts afresh");
    }

    /// §10.8.1: a raised box lifts its content off the baseline; a `top`
    /// box taller than the line grows it downward, a `bottom` one upward.
    #[test]
    fn shifts_and_aligned_subtrees() {
        let mut f = Frames::new(rows(0, 0));
        f.enter(rows(0, 0), BoxAlign::Shift(1));
        let sup = f.current();
        f.leave();
        f.mark(sup);
        let s = f.settle();
        assert_eq!((s.baseline(), s.height, f.row(&s, sup)), (1, 2, 0));

        for (align, baseline) in [(BoxAlign::Top, 0), (BoxAlign::Bottom, 2)] {
            let mut f = Frames::new(rows(0, 0));
            let tall = f.add(rows(0, 2), align);
            f.mark(tall);
            let s = f.settle();
            assert_eq!((s.baseline(), s.height, f.row(&s, tall)), (baseline, 3, 0));
        }
    }
}
