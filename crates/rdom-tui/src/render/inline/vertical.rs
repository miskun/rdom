//! Line box heights (CSS 2.1 §10.8): how tall each line is and where
//! its atoms sit in it.
//!
//! Text and generated content are one row tall and share the line's
//! baseline row. An atomic inline block brings its margin box (CSS 2.1
//! §10.8: "the height of the margin box" for an inline-block) and its
//! baseline (§10.8.1): the text row of its last line box
//! (`layout_pass::baselines`, the lines layout packs), so a `height`
//! larger than the content leaves the baseline on the content's last
//! line, as in a browser — or, with no in-flow line boxes or with
//! `overflow` other than `visible`, its bottom margin edge (its last
//! row). `vertical-align` is `baseline` (its initial value; the property
//! is C9-VERTICAL-ALIGN): the line's baseline row is the largest number
//! of rows any piece of content has above its baseline, and its height
//! adds the largest number below.
//!
//! Text sits on the baseline row too, and each inline box on the line —
//! the block's strut among them — adds its leading above and below it
//! (its `line-height`, `packer::frames`).
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
    pub(super) fn above(self) -> u16 {
        self.baseline
    }

    /// Rows of its margin box below its baseline row.
    pub(super) fn below(self) -> u16 {
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
    // CSS 2.1 §10.8.1: an inline block's baseline is its last line box's;
    // §17.5.3: an inline table's is its first row's.
    let table = computed.flow == crate::layout::Flow::Table;
    let last =
        crate::render::layout_pass::baselines::content_rows(dom, id, &computed, width, cb_width)
            .map(|(first, last)| if table { first } else { last });
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

/// Where an atom of a line is: an element's fragment, or an atomic
/// pseudo-element's generated fragment, by index in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AtomAt {
    Fragment(usize),
    Generated(usize),
}

/// Place each fragment of a settled line on it (CSS 2.1 §10.8): text and
/// generated text on their inline box's baseline row, each atom (`atoms`:
/// where it is, its rows, its inline box) with its baseline on its box's —
/// `row_of` the row the packer settled an inline box's baseline on.
pub(super) fn settle_line(
    fragments: &mut [InlineFragment],
    generated: &mut [GeneratedFragment],
    atoms: &[(AtomAt, AtomRows, u32)],
    row_of: impl Fn(u32) -> u16,
) {
    for f in fragments.iter_mut().filter(|f| !f.atomic) {
        f.y = row_of(f.frame);
        f.height = 1;
    }
    for g in generated.iter_mut().filter(|g| !g.is_atom()) {
        g.y = row_of(g.frame);
    }
    for &(at, a, frame) in atoms {
        let y = row_of(frame) - a.above() + a.margin_top;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdom_core::Dom;

    fn fragment(node: NodeId, atomic: bool, frame: u32) -> InlineFragment {
        let mut f = if atomic {
            InlineFragment::atom(node, 0, 1, 1)
        } else {
            InlineFragment::text(node, node, 0, 0, "a")
        };
        f.frame = frame;
        f
    }

    /// CSS 2.1 §10.8: text sits on its box's baseline row; an atom's
    /// baseline lands on its box's, below its top margin.
    #[test]
    fn fragments_sit_on_their_boxes_rows() {
        let dom: Dom<TuiExt> = Dom::new();
        let n = dom.root();
        let bordered = AtomRows {
            margin_top: 1,
            height: 3,
            margin_bottom: 0,
            baseline: 2,
        };
        let mut line = [
            fragment(n, false, 0),
            fragment(n, true, 1),
            fragment(n, false, 2),
        ];
        let rows = |frame: u32| [3, 4, 1][frame as usize];
        settle_line(
            &mut line,
            &mut [],
            &[(AtomAt::Fragment(1), bordered, 1)],
            rows,
        );
        assert_eq!((line[0].y, line[0].height), (3, 1));
        assert_eq!(
            (line[1].y, line[1].height),
            (3, 3),
            "baseline 4, 2 rows of margin box above it, 1 of them margin"
        );
        assert_eq!(line[2].y, 1, "a raised box's text");
    }
}
