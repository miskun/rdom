//! Line box heights (CSS 2.1 §10.8): how tall each line is and where
//! its atoms sit in it.
//!
//! Text and generated content are one row tall and share the line's
//! baseline row. An atomic inline block brings its margin box (CSS 2.1
//! §10.8: "the height of the margin box" for an inline-block) and its
//! baseline (§10.8.1): the row of its last line box — counted here as
//! the last row of its content (`intrinsic` content height), so a
//! `height` larger than the content leaves the baseline on the content's
//! last line, as in a browser — or, with no in-flow line boxes or with
//! `overflow` other than `visible`, its bottom margin edge (its last
//! row). `vertical-align` is `baseline` (its initial value; the property
//! is C9-VERTICAL-ALIGN): the line's baseline row is the largest number
//! of rows any piece of content has above its baseline, and its height
//! adds the largest number below.
//!
//! A negative vertical margin on an atom counts as zero here: rows are
//! whole, and an atom pulled above its line box would overlap the
//! previous line (DIVERGENCES §2, Layout).

use rdom_core::{Dom, NodeId};

use super::InlineFragment;
use crate::ext::TuiExt;
use crate::layout::{Direction, Overflow};
use crate::render::layout_pass::intrinsic;

/// An atom's block-axis geometry in its line, measured before the
/// line is settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AtomRows {
    /// Rows of top margin above its border box.
    margin_top: u16,
    /// Its border-box height.
    pub(super) height: u16,
    /// Rows of bottom margin below its border box.
    margin_bottom: u16,
    /// Its baseline row, counted from its margin-box top.
    baseline: u16,
}

impl AtomRows {
    /// Rows of its margin box above its baseline row.
    fn above(self) -> u16 {
        self.baseline
    }

    /// Rows of its margin box below its baseline row.
    fn below(self) -> u16 {
        self.margin_top
            .saturating_add(self.height)
            .saturating_add(self.margin_bottom)
            .saturating_sub(self.baseline)
            .saturating_sub(1)
    }
}

/// Measure the inline block `id` laid out `width` cells wide in a line
/// whose content box (its containing block) is `cb_width` wide.
pub(super) fn atom_rows(dom: &Dom<TuiExt>, id: NodeId, width: u16, cb_width: u16) -> AtomRows {
    let Some(computed) = dom.node(id).ext().and_then(|e| e.computed.clone()) else {
        return AtomRows {
            margin_top: 0,
            height: 1,
            margin_bottom: 0,
            baseline: 0,
        };
    };
    let height = intrinsic::intrinsic_size(dom, id, Direction::Column, width, cb_width);
    let margin = |m: &crate::layout::MarginValue| m.resolve(cb_width).max(0) as u16;
    let margin_top = margin(&computed.margin.top);
    let margin_bottom = margin(&computed.margin.bottom);
    let outer = margin_top
        .saturating_add(height)
        .saturating_add(margin_bottom);
    let bottom_edge = outer.saturating_sub(1);
    let chrome_top = computed
        .border
        .top
        .cells()
        .saturating_add(computed.padding.top.resolve(cb_width));
    let chrome = chrome_top
        .saturating_add(computed.border.bottom.cells())
        .saturating_add(computed.padding.bottom.resolve(cb_width));
    let content_rows = intrinsic::content_max_size(dom, id, Direction::Column, width, cb_width)
        .saturating_sub(chrome);
    let visible =
        computed.overflow_x == Overflow::Visible && computed.overflow_y == Overflow::Visible;
    let baseline = if visible && content_rows > 0 {
        (margin_top + chrome_top + content_rows - 1).min(bottom_edge)
    } else {
        bottom_edge
    };
    AtomRows {
        margin_top,
        height,
        margin_bottom,
        baseline,
    }
}

/// Settle one line: place each fragment on it — text on the baseline
/// row, each atom (`atoms`: its fragment's index and rows) with its
/// baseline there — and return the line's `(baseline, height)`.
pub(super) fn settle_line(
    fragments: &mut [InlineFragment],
    atoms: &[(usize, AtomRows)],
) -> (u16, u16) {
    let above = atoms.iter().map(|(_, a)| a.above()).max().unwrap_or(0);
    let below = atoms.iter().map(|(_, a)| a.below()).max().unwrap_or(0);
    for f in fragments.iter_mut() {
        f.y = above;
        f.height = 1;
    }
    for &(i, a) in atoms {
        let f = &mut fragments[i];
        f.y = above - a.above() + a.margin_top;
        f.height = a.height;
    }
    (above, above.saturating_add(1).saturating_add(below))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdom_core::Dom;

    fn fragment(node: NodeId, atomic: bool) -> InlineFragment {
        InlineFragment {
            node,
            text_node: node,
            source_byte_offset: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            text: String::new(),
            atomic,
        }
    }

    /// CSS 2.1 §10.8: text alone makes a one-row line; a three-row atom
    /// whose baseline is its middle row puts the text there and grows
    /// the line to three rows; an empty atom (baseline = its bottom
    /// edge) rises above the text.
    #[test]
    fn a_line_is_as_tall_as_its_atoms_around_the_baseline() {
        let dom: Dom<TuiExt> = Dom::new();
        let n = dom.root();
        let mut text_only = [fragment(n, false)];
        assert_eq!(settle_line(&mut text_only, &[]), (0, 1));

        let bordered = AtomRows {
            margin_top: 0,
            height: 3,
            margin_bottom: 0,
            baseline: 1,
        };
        let mut line = [fragment(n, false), fragment(n, true)];
        assert_eq!(settle_line(&mut line, &[(1, bordered)]), (1, 3));
        assert_eq!((line[0].y, line[0].height), (1, 1));
        assert_eq!((line[1].y, line[1].height), (0, 3));

        let empty = AtomRows {
            margin_top: 1,
            height: 2,
            margin_bottom: 0,
            baseline: 2,
        };
        let mut line = [fragment(n, true), fragment(n, false)];
        assert_eq!(settle_line(&mut line, &[(0, empty)]), (2, 3));
        assert_eq!((line[0].y, line[0].height), (1, 2), "below its top margin");
        assert_eq!(line[1].y, 2);
    }
}
