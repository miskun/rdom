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

use super::{GeneratedFragment, InlineFragment};
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::render::layout_pass::intrinsic;

/// An atom's block-axis geometry in its line, measured before the
/// line is settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AtomRows {
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
    /// An atom whose rows a width measurement does not ask for
    /// (`LinePacker::measuring`): one row, its baseline.
    pub(crate) const UNMEASURED: Self = AtomRows {
        margin_top: 0,
        height: 1,
        margin_bottom: 0,
        baseline: 0,
    };

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
        return AtomRows::UNMEASURED;
    };
    let height = intrinsic::intrinsic_size(dom, id, Direction::Column, width, cb_width);
    let last = content_rows(dom, id, &computed, width, cb_width).map(|(_, last)| last);
    AtomRows::of(&computed, height, cb_width, last)
}

impl AtomRows {
    /// The rows of an atom styled `computed`, its border box `height`
    /// rows tall in a line whose content box is `cb_width` wide, whose
    /// last content row — counted from its border-box top — is `last`:
    /// its vertical margins (a negative one as 0), and its baseline that
    /// row, or its bottom margin edge with no content row or as a scroll
    /// container (CSS Box Alignment 3 §9.1: a scroll container's — not a
    /// `clip` box's — baseline is its margin edge).
    pub(crate) fn of(
        computed: &crate::style::ComputedStyle,
        height: u16,
        cb_width: u16,
        last: Option<u16>,
    ) -> Self {
        let margin = |m: &crate::layout::MarginValue| m.resolve(cb_width).max(0) as u16;
        let margin_top = margin(&computed.margin.top);
        let margin_bottom = margin(&computed.margin.bottom);
        let outer = margin_top
            .saturating_add(height)
            .saturating_add(margin_bottom);
        let bottom_edge = outer.saturating_sub(1);
        let baseline = match last {
            Some(last) if !computed.is_scroll_container() => (margin_top + last).min(bottom_edge),
            _ => bottom_edge,
        };
        AtomRows {
            margin_top,
            height,
            margin_bottom,
            baseline,
        }
    }

    /// Its border-box height.
    pub(crate) fn height(self) -> u16 {
        self.height
    }
}

/// The rows of `id`'s content, laid out `width` cells wide, counted
/// from its border-box top: its first and last content rows — the rows
/// of its first and last line boxes, as a cell grid has them (one
/// baseline per row) — or `None` when it has no content rows.
pub(crate) fn content_rows(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &crate::style::ComputedStyle,
    width: u16,
    cb_width: u16,
) -> Option<(u16, u16)> {
    let chrome_top = computed
        .border
        .top
        .cells()
        .saturating_add(computed.padding.top.resolve(cb_width));
    let chrome = chrome_top
        .saturating_add(computed.border.bottom.cells())
        .saturating_add(computed.padding.bottom.resolve(cb_width));
    let rows = intrinsic::content_max_size(dom, id, Direction::Column, width, cb_width)
        .saturating_sub(chrome);
    (rows > 0).then(|| (chrome_top, chrome_top + rows - 1))
}

/// Where an atom of a line is: an element's fragment, or an atomic
/// pseudo-element's generated fragment, by index in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AtomAt {
    Fragment(usize),
    Generated(usize),
}

/// Settle one line: place each fragment on it — text on the baseline
/// row, each atom (`atoms`: where it is and its rows) with its baseline
/// there — and return the line's `(baseline, height)`.
pub(super) fn settle_line(
    fragments: &mut [InlineFragment],
    generated: &mut [GeneratedFragment],
    atoms: &[(AtomAt, AtomRows)],
) -> (u16, u16) {
    let above = atoms.iter().map(|(_, a)| a.above()).max().unwrap_or(0);
    let below = atoms.iter().map(|(_, a)| a.below()).max().unwrap_or(0);
    for f in fragments.iter_mut() {
        f.y = above;
        f.height = 1;
    }
    for &(at, a) in atoms {
        let y = above - a.above() + a.margin_top;
        match at {
            AtomAt::Fragment(i) => {
                let f = &mut fragments[i];
                f.y = y;
                f.height = a.height;
            }
            AtomAt::Generated(i) => {
                if let Some(atom) = generated[i].atom.as_mut() {
                    atom.y = y;
                    atom.height = a.height;
                }
            }
        }
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
            map: None,
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
        assert_eq!(settle_line(&mut text_only, &mut [], &[]), (0, 1));

        let bordered = AtomRows {
            margin_top: 0,
            height: 3,
            margin_bottom: 0,
            baseline: 1,
        };
        let mut line = [fragment(n, false), fragment(n, true)];
        assert_eq!(
            settle_line(&mut line, &mut [], &[(AtomAt::Fragment(1), bordered)]),
            (1, 3)
        );
        assert_eq!((line[0].y, line[0].height), (1, 1));
        assert_eq!((line[1].y, line[1].height), (0, 3));

        let empty = AtomRows {
            margin_top: 1,
            height: 2,
            margin_bottom: 0,
            baseline: 2,
        };
        let mut line = [fragment(n, true), fragment(n, false)];
        assert_eq!(
            settle_line(&mut line, &mut [], &[(AtomAt::Fragment(0), empty)]),
            (2, 3)
        );
        assert_eq!((line[0].y, line[0].height), (1, 2), "below its top margin");
        assert_eq!(line[1].y, 2);
    }
}
