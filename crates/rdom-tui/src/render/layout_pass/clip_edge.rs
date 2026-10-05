//! A box's overflow clip edges (CSS Overflow 3 §3): where its content
//! stops painting, and stops counting toward an ancestor's scrollable
//! overflow, on each axis. Paint, hit-testing and the scrollable-extent
//! walk share it.

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, VisualBox};
use crate::style::ComputedStyle;

/// The `[start, end)` cells a box clips its content to on each axis,
/// `None` on an axis it does not clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipEdges {
    pub(crate) x: Option<(i32, i32)>,
    pub(crate) y: Option<(i32, i32)>,
}

impl ClipEdges {
    /// No clipping on either axis.
    pub(crate) const NONE: Self = Self { x: None, y: None };

    /// The edges of the box `ext` styled `c`: a scroll container's
    /// scrollport — the padding box less its scrollbar gutters
    /// (`scrollport`) — on both axes (§3.1 makes both of its axes clip;
    /// §5.2 puts the gutters outside the padding edge, so no content
    /// paints in a gutter whether or not a bar is shown there); on
    /// each `overflow: clip` axis the overflow clip edge — the
    /// `overflow-clip-margin` box outset by its margin (§3.2); none on a
    /// `visible` axis.
    pub(crate) fn of(ext: &TuiExt, c: &ComputedStyle) -> Self {
        if !c.clips_overflow() {
            return Self::NONE;
        }
        let (edge, grow) = if c.is_scroll_container() {
            (super::scrollport::scrollport_of(ext, c), 0)
        } else {
            let padding_box = super::geometry::compute_padding_box(ext.layout, c.border);
            let m = c.overflow_clip_margin;
            let base = match m.visual_box {
                VisualBox::BorderBox => ext.layout,
                VisualBox::PaddingBox => padding_box,
                VisualBox::ContentBox => ext.content_layout,
            };
            (base, i32::from(m.margin))
        };
        let axis = |clips: bool, start: i32, len: u16| {
            clips.then(|| (start - grow, start + i32::from(len) + grow))
        };
        Self {
            x: axis(c.overflow_x.clips(), edge.x, edge.width),
            y: axis(c.overflow_y.clips(), edge.y, edge.height),
        }
    }

    /// [`of`](Self::of) for the element `id`, its line clamp included: a
    /// line-clamp container's content ends at its clamp point on the
    /// block axis, whatever its `overflow` (CSS Overflow 4 §4.4), for
    /// paint, hit-testing and the scrollable overflow alike.
    pub(crate) fn of_element(
        dom: &rdom_core::Dom<TuiExt>,
        id: rdom_core::NodeId,
        ext: &TuiExt,
        c: &ComputedStyle,
    ) -> Self {
        let edges = Self::of(ext, c);
        if !c.line_clamp_container {
            return edges;
        }
        match super::line_clamp::clamp_point(dom, id) {
            Some(point) => edges.narrow(Self {
                x: None,
                y: Some((i32::MIN / 2, point.bottom)),
            }),
            None => edges,
        }
    }

    /// The edges of both: each axis the narrower.
    pub(crate) fn narrow(self, other: Self) -> Self {
        let both = |a: Option<(i32, i32)>, b: Option<(i32, i32)>| match (a, b) {
            (Some(a), Some(b)) => Some((a.0.max(b.0), a.1.min(b.1))),
            (a, None) => a,
            (None, b) => b,
        };
        Self {
            x: both(self.x, other.x),
            y: both(self.y, other.y),
        }
    }

    /// `rect` cut to the edges, `None` when nothing of it is inside.
    pub(crate) fn cut(self, rect: LayoutRect) -> Option<LayoutRect> {
        let axis = |edge: Option<(i32, i32)>, start: i32, len: u16| {
            let end = start + i32::from(len);
            let (s, e) = edge.map_or((start, end), |(a, b)| (start.max(a), end.min(b)));
            (e > s || (len == 0 && e == s)).then_some((s, e))
        };
        let (x0, x1) = axis(self.x, rect.x, rect.width)?;
        let (y0, y1) = axis(self.y, rect.y, rect.height)?;
        let len = |a: i32, b: i32| (b - a).clamp(0, i32::from(u16::MAX)) as u16;
        Some(LayoutRect::new(x0, y0, len(x0, x1), len(y0, y1)))
    }
}
