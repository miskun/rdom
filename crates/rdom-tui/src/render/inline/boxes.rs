//! The inline layout's data model: an [`InlineLayout`] is a stack of
//! [`LineBox`]es, each holding [`InlineFragment`]s (text runs and
//! atomic inline blocks) and [`GeneratedFragment`]s (`::before` /
//! `::after` runs).
//!
//! ## Vertical geometry (CSS 2.1 §10.8)
//!
//! A line box is as tall as the content on it: one row for text, more
//! when an atomic inline block on it is taller. Its text sits on one
//! row of it, the *baseline* row; each atom is placed so that its own
//! baseline (CSS 2.1 §10.8.1 — its last line box, or its bottom margin
//! edge) lands on that row, which is the `vertical-align: baseline`
//! initial value (`vertical_align`). Lines stack without gaps —
//! `lines[i + 1].top == lines[i].top + lines[i].height` — except where a
//! line too narrow beside a float moved down past it (CSS 2.1 §9.5): its
//! `top` is then lower, and the rows between belong to no line.

use rdom_core::NodeId;

use crate::ext::PseudoSlot;

/// One visible chunk of text painted contiguously on a single line
/// with a single owner element + source text node. An inline
/// element whose text wraps produces multiple fragments (one per
/// line). A whitespace-collapsed separator ("a <b>bold</b>") is
/// also a single fragment whose text is `" "`.
///
/// **Atomic inline-block fragments** (`atomic = true`) carry a
/// `Display::InlineBlock` element participating in IFC. Their
/// `text` is empty; their `width` is the box's intrinsic main-
/// axis size including UA pseudo content (`<button>`'s `[ … ]`),
/// their `height` its block size. The layout pass lays the element
/// out at that rect and paint paints it there as a box — background,
/// border, content — at its turn in the line; selection skips them;
/// hit-test routes to `node`.
///
/// `#[non_exhaustive]`: fields will be added (`vertical-align`,
/// C9-VERTICAL-ALIGN), so outside rdom-tui a fragment is built with
/// [`InlineFragment::text`] or [`InlineFragment::atom`], and its public
/// fields are read or set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineFragment {
    /// The direct element parent of the source text, or — for
    /// `atomic = true` fragments — the inline-block element itself.
    /// Click / hover routes here.
    pub node: NodeId,
    /// The source `Text` node whose data this fragment renders. For
    /// whitespace-collapsed separators, this is the text node that
    /// contained the first collapsed whitespace byte. For
    /// `atomic = true` fragments, set to the inline-block element
    /// (sentinel — there's no source text node).
    pub text_node: NodeId,
    /// Byte offset in `text_node`'s data where this fragment's
    /// first grapheme sits. The runtime's `position_at` walks
    /// fragment graphemes from `x` to compute the hit position.
    /// `0` for atomic fragments.
    pub source_byte_offset: usize,
    /// X offset from the IFC block's content area left edge — negative
    /// for a fragment left of it (a line wider than its `rtl` box starts
    /// at the right edge and overflows the left one).
    pub x: i32,
    /// Rows from the top of its line box to the fragment's top: the
    /// line's baseline row for text, the border-box top for an atom.
    pub y: u16,
    /// Visible cell width of `text` (or, for atomic fragments,
    /// the inline-block's intrinsic main-axis content size).
    pub width: u16,
    /// Rows the fragment occupies: 1 for text, the border-box height
    /// for an atom.
    pub height: u16,
    /// Normalized text to paint. No control characters; no leading /
    /// trailing whitespace when this fragment brackets a line.
    /// Empty for `atomic = true` fragments.
    pub text: String,
    /// True iff this fragment is an atomic inline-block box
    /// (`Display::InlineBlock` participating in IFC). See the type
    /// doc for the full contract.
    pub atomic: bool,
}

impl InlineFragment {
    /// A text fragment: `text` from byte `source_byte_offset` of
    /// `text_node`, owned by `node`, at `x` on its line's baseline row —
    /// one row tall and as wide as `text`'s visible cells.
    pub fn text(
        node: NodeId,
        text_node: NodeId,
        source_byte_offset: usize,
        x: i32,
        text: impl Into<String>,
    ) -> Self {
        let text = text.into();
        let width = unicode_width::UnicodeWidthStr::width(text.as_str()).min(usize::from(u16::MAX));
        InlineFragment {
            node,
            text_node,
            source_byte_offset,
            x,
            y: 0,
            width: width as u16,
            height: 1,
            text,
            atomic: false,
        }
    }

    /// An atomic inline's fragment: the box `node`, `width` × `height`
    /// cells, at `x` from the top of its line (set `y` to place it
    /// lower). No text; `text_node` is `node`.
    pub fn atom(node: NodeId, x: i32, width: u16, height: u16) -> Self {
        InlineFragment {
            node,
            text_node: node,
            source_byte_offset: 0,
            x,
            y: 0,
            width,
            height,
            text: String::new(),
            atomic: true,
        }
    }
}

/// A run of a host's static `::before` / `::after` content on one
/// line. CSS 2.1 §12.1: generated content is an inline box, the first
/// / last child of its host, so the packer lays it out with the text —
/// it wraps, and the text after it starts past it. It sits on its
/// line's baseline row, as text does.
///
/// Generated content has no DOM node and no DOM position, so it is kept
/// apart from [`LineBox::fragments`]: hit-testing, the caret,
/// selection highlight and copy only ever see text and atoms, and a
/// click on a generated cell clamps to the nearest text position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFragment {
    /// The element whose pseudo-element this is (paint reads its
    /// `computed_before` / `computed_after`).
    pub host: NodeId,
    /// Which of the host's pseudo-elements this run belongs to.
    pub slot: PseudoSlot,
    /// X offset from the inline flow's content-area left edge, negative
    /// left of it (as [`InlineFragment::x`]).
    pub x: i32,
    /// Visible cell width of `text`.
    pub width: u16,
    /// The normalized generated text on this line.
    pub text: String,
}

/// One line of inline content.
///
/// `#[non_exhaustive]`: fields will be added (C9-VERTICAL-ALIGN), so
/// outside rdom-tui a line is built with [`LineBox::new`] or from
/// [`LineBox::default`] (an empty one-row line at the top) with its
/// public fields set:
///
/// ```compile_fail
/// let line = rdom_tui::render::LineBox {
///     fragments: Vec::new(),
///     generated: Vec::new(),
///     width: 0,
///     top: 0,
///     height: 1,
///     baseline: 0,
/// };
/// ```
///
/// ```
/// let mut line = rdom_tui::render::LineBox::default();
/// line.top = 2;
/// assert_eq!(line.text_row(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LineBox {
    /// Fragments in left-to-right order, each non-overlapping.
    pub fragments: Vec<InlineFragment>,
    /// Generated-content runs on this line, left to right. They occupy
    /// cells between / around `fragments` — never overlapping them.
    pub generated: Vec<GeneratedFragment>,
    /// Total visible width of this line, generated content included
    /// (≤ content width unless a single word overflowed).
    pub width: u16,
    /// Rows from the top of the inline layout to the top of this line.
    pub top: u16,
    /// Rows this line box spans: 1, or more when an atom on it is
    /// taller (CSS 2.1 §10.8).
    pub height: u16,
    /// The row of this line its text and generated content sit on,
    /// counted from [`top`](Self::top): its baseline (CSS 2.1 §10.8).
    pub baseline: u16,
}

impl Default for LineBox {
    /// An empty one-row line at the top of its layout.
    fn default() -> Self {
        LineBox {
            fragments: Vec::new(),
            generated: Vec::new(),
            width: 0,
            top: 0,
            height: 1,
            baseline: 0,
        }
    }
}

impl LineBox {
    /// A one-row line of `fragments`, `width` cells wide, at row `top`
    /// of its layout, with no generated content.
    pub fn new(fragments: Vec<InlineFragment>, width: u16, top: u16) -> Self {
        LineBox {
            fragments,
            width,
            top,
            ..LineBox::default()
        }
    }

    /// The row its text sits on, counted from the top of the inline
    /// layout.
    pub fn text_row(&self) -> u16 {
        self.top.saturating_add(self.baseline)
    }

    /// The row after its last, counted from the top of the inline
    /// layout: the next line's `top`.
    pub fn bottom(&self) -> u16 {
        self.top.saturating_add(self.height)
    }

    /// True when `fragment`, one of this line's, occupies `row` (counted
    /// from the top of the inline layout): the baseline row for text,
    /// any row of its border box for an atom.
    pub fn covers(&self, fragment: &InlineFragment, row: u16) -> bool {
        let top = self.top.saturating_add(fragment.y);
        row >= top && row < top.saturating_add(fragment.height)
    }
}

/// Full inline layout for an IFC block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLayout {
    pub lines: Vec<LineBox>,
    /// The content width this layout was packed for. Paint reuses it
    /// to know where to clip overflowing fragments.
    pub content_width: u16,
}

impl InlineLayout {
    /// Height in rows: the line boxes' heights stacked.
    pub fn height(&self) -> u16 {
        self.lines.last().map_or(0, LineBox::bottom)
    }

    /// The index of the line box spanning `row` (counted from the top
    /// of the layout), or `None` past the last line or in the rows a line
    /// moved down past a float left empty.
    pub fn line_at_row(&self, row: u16) -> Option<usize> {
        // Lines are sorted by `top`.
        let i = self.lines.partition_point(|l| l.bottom() <= row);
        (i < self.lines.len() && self.lines[i].top <= row).then_some(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(top: u16, height: u16) -> LineBox {
        LineBox {
            fragments: Vec::new(),
            generated: Vec::new(),
            width: 0,
            top,
            height,
            baseline: 0,
        }
    }

    /// Rows map onto the stacked line boxes, a tall line spanning all
    /// of its rows; past the last line there is none.
    #[test]
    fn line_at_row_follows_the_line_heights() {
        let layout = InlineLayout {
            lines: vec![line(0, 1), line(1, 3), line(4, 1)],
            content_width: 10,
        };
        assert_eq!(layout.height(), 5);
        let found: Vec<_> = (0..6).map(|r| layout.line_at_row(r)).collect();
        assert_eq!(
            found,
            vec![Some(0), Some(1), Some(1), Some(1), Some(2), None]
        );
    }
}
