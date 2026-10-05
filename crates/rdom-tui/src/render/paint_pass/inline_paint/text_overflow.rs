//! `text-overflow` at paint (CSS Overflow 4 §3): on each line box of a
//! block container that clips its inline axis, the content past an edge
//! with a marker (`…` or a string) is hidden whole character by whole
//! character — an atomic inline as one — until the marker fits beside
//! what is left, and the marker paints there. Layout, hit-testing and
//! copying never see the cut: copying an ellipsed line copies all of it.

use rdom_core::{Dom, NodeId};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::{TextDirection, TextOverflowSide};
use crate::render::Style;
use crate::render::inline::LineBox;

/// What `text-overflow` asks of one block's lines: the cells of its
/// content box on the inline axis (`[left, right)`, in viewport columns,
/// unscrolled — the edges lines overflow), the line-left and line-right
/// edges' values, and the marker's style (the block's).
pub(super) struct Marking {
    window: (i32, i32),
    left: TextOverflowSide,
    right: TextOverflowSide,
    pub(super) style: Style,
}

impl Marking {
    /// `block`'s marking, `None` when its `text-overflow` clips both edges
    /// or its inline axis does not clip (§3: the property applies to a
    /// block with `overflow` other than `visible`).
    pub(super) fn of(dom: &Dom<TuiExt>, block: NodeId) -> Option<Self> {
        let ext = dom.node(block).ext()?;
        let c = ext.computed.as_deref()?;
        if !c.overflow_x.clips() || c.text_overflow.is_clip() {
            return None;
        }
        let (left, right) = c
            .text_overflow
            .line_sides(c.text_direction == TextDirection::Rtl);
        let content = ext.content_layout;
        Some(Self {
            window: (content.x, content.x + i32::from(content.width)),
            left: left.clone(),
            right: right.clone(),
            style: super::super::text::glyph_style_from_computed(c),
        })
    }
}

/// How one line paints under a [`Marking`]: only the cells in
/// `[left, right)` of its content, and the markers at their columns.
pub(super) struct LineCut<'m> {
    pub(super) left: i32,
    pub(super) right: i32,
    pub(super) markers: Vec<(i32, &'m str)>,
}

impl LineCut<'_> {
    /// Whether the cells `[start, end)` paint.
    pub(super) fn keeps(&self, start: i32, end: i32) -> bool {
        start >= self.left && end <= self.right
    }
}

/// The cut of `line`, laid out from column `origin_x`, under `marking`.
/// An edge the line does not overflow, or whose value is `clip`, is not
/// cut (the box's clip cuts it). §3: "the first character or atomic
/// inline-level element on a line must be clipped rather than ellipsed" —
/// when not even it fits beside the marker, that edge clips instead.
pub(super) fn cut_line<'m>(line: &LineBox, origin_x: i32, marking: &'m Marking) -> LineCut<'m> {
    let pieces = pieces(line, origin_x);
    let mut cut = LineCut {
        left: i32::MIN,
        right: i32::MAX,
        markers: Vec::new(),
    };
    let (Some(first), Some(last)) = (pieces.first(), pieces.iter().map(|p| p.1).max()) else {
        return cut;
    };
    let (start, end) = (first.0, last);
    let (window_left, window_right) = marking.window;
    if end > window_right
        && let Some(marker) = marking.right.marker()
    {
        let limit = window_right - width(marker);
        // The cells kept run from the line's start to the last whole
        // piece that ends by `limit`.
        let kept = pieces.iter().filter(|p| p.1 <= limit).map(|p| p.1).max();
        if let Some(edge) = kept.filter(|_| first.1 <= limit) {
            cut.right = edge;
            cut.markers.push((edge, marker));
        }
    }
    if start < window_left
        && let Some(marker) = marking.left.marker()
    {
        let w = width(marker);
        let limit = window_left + w;
        let kept = pieces.iter().filter(|p| p.0 >= limit).map(|p| p.0).min();
        let last_fits = pieces.iter().any(|p| p.1 == end && p.0 >= limit);
        if let Some(edge) = kept.filter(|&e| last_fits && e < cut.right) {
            cut.left = edge;
            cut.markers.push((edge - w, marker));
        }
    }
    cut
}

/// The marker's width in cells.
fn width(marker: &str) -> i32 {
    i32::try_from(UnicodeWidthStr::width(marker)).unwrap_or(i32::MAX)
}

/// The cells `[start, end)` of each grapheme of the line's text and
/// generated content and of each atom, in viewport columns, by start.
fn pieces(line: &LineBox, origin_x: i32) -> Vec<(i32, i32)> {
    fn runs(out: &mut Vec<(i32, i32)>, mut at: i32, text: &str) {
        for g in text.graphemes(true) {
            let w = i32::try_from(UnicodeWidthStr::width(g)).unwrap_or(0);
            if w > 0 {
                out.push((at, at + w));
            }
            at += w;
        }
    }
    let mut out = Vec::new();
    for f in &line.fragments {
        let x = origin_x + i32::from(f.x);
        if f.atomic {
            out.push((x, x + i32::from(f.width)));
        } else {
            runs(&mut out, x, &f.text);
        }
    }
    for g in &line.generated {
        runs(&mut out, origin_x + i32::from(g.x), &g.text);
    }
    out.sort_unstable();
    out
}
