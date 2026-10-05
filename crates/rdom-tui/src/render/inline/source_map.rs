//! A text fragment's painted text against its source text.
//!
//! [`InlineFragment::text`] is what paint draws. Most of the time it is
//! the source text itself, byte for byte, grapheme for grapheme. Where
//! layout renders a source grapheme as something else — a soft hyphen
//! that shows `-` at a line break or nothing elsewhere (CSS Text 3 §6.1),
//! a tab as the spaces to its tab stop (§4.2), a case-mapped or
//! full-width letter (§2.1), a justified space (§7.3) — the fragment
//! carries a [`SourceMap`]: per source grapheme, its source bytes and the
//! bytes of `text` it renders as. Every reader that maps between cells
//! and source positions — the caret, hit-testing, the selection
//! highlight, word and line movement — goes through [`Unit`]s, so they
//! agree whatever the rendering; copy reads the DOM text.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::InlineFragment;

/// Per source grapheme of a fragment, `(source bytes, text bytes)`, in
/// order: the fragment's `text` is the concatenation of the rendered
/// pieces, its source the concatenation of the source graphemes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SourceMap(Vec<(u32, u32)>);

impl SourceMap {
    /// A map of `units`, `(source bytes, text bytes)` each.
    pub(crate) fn new(units: Vec<(u32, u32)>) -> Self {
        SourceMap(units)
    }

    /// The map of `text` rendered verbatim: one unit per grapheme.
    pub(crate) fn verbatim(text: &str) -> Self {
        SourceMap(
            text.graphemes(true)
                .map(|g| (g.len() as u32, g.len() as u32))
                .collect(),
        )
    }

    /// Append `other`'s units.
    pub(crate) fn extend(&mut self, other: &SourceMap) {
        self.0.extend_from_slice(&other.0);
    }

    /// The source bytes the map covers.
    fn source_len(&self) -> usize {
        self.0.iter().map(|&(s, _)| s as usize).sum()
    }

    /// The units, mutably — a renderer that changes a unit's text (the
    /// hyphen a broken soft hyphen shows) adjusts its text bytes here.
    pub(crate) fn units_mut(&mut self) -> &mut Vec<(u32, u32)> {
        &mut self.0
    }
}

/// One source grapheme of a fragment as rendered: its source bytes
/// (from the fragment's `source_byte_offset`), the text it paints, and
/// that text's cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit<'a> {
    pub(crate) source: Range<usize>,
    pub(crate) text: &'a str,
    pub(crate) width: u16,
}

impl InlineFragment {
    /// The bytes of `text_node`'s data this fragment renders, from
    /// [`source_byte_offset`](Self::source_byte_offset): `text.len()`
    /// unless layout renders some of its source as something else (a soft
    /// hyphen, a tab, `text-transform`, justification).
    pub fn source_len(&self) -> usize {
        match &self.map {
            None => self.text.len(),
            Some(map) => map.source_len(),
        }
    }

    /// The fragment's source graphemes as rendered, in order.
    pub(crate) fn units(&self) -> Box<dyn Iterator<Item = Unit<'_>> + '_> {
        let width = |t: &str| UnicodeWidthStr::width(t).min(usize::from(u16::MAX)) as u16;
        match &self.map {
            None => Box::new(self.text.grapheme_indices(true).map(move |(i, g)| Unit {
                source: i..i + g.len(),
                text: g,
                width: width(g),
            })),
            Some(map) => {
                let (mut source, mut text) = (0usize, 0usize);
                Box::new(map.0.iter().map(move |&(s, t)| {
                    let (s, t) = (s as usize, t as usize);
                    let unit = Unit {
                        source: source..source + s,
                        text: &self.text[text..text + t],
                        width: width(&self.text[text..text + t]),
                    };
                    source += s;
                    text += t;
                    unit
                }))
            }
        }
    }

    /// Cells before source byte `target` (from the fragment's start): the
    /// units that start before it, a target inside one rounding up past
    /// it (carets and selection ends land on grapheme edges); past the
    /// end, the whole width.
    pub(crate) fn cells_before_source(&self, target: usize) -> u16 {
        let mut cells: u16 = 0;
        for unit in self.units() {
            if unit.source.start >= target {
                return cells;
            }
            cells = cells.saturating_add(unit.width);
        }
        cells
    }

    /// The source byte (from the fragment's start) of the unit whose cells
    /// hold cell `target` — a cell inside a wide unit snaps to its start —
    /// or the fragment's source length past its last cell.
    pub(crate) fn source_at_cell(&self, target: u16) -> usize {
        let mut cells: u16 = 0;
        for unit in self.units() {
            cells = cells.saturating_add(unit.width);
            if target < cells {
                return unit.source.start;
            }
        }
        self.source_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fragment(text: &str, map: Option<Vec<(u32, u32)>>) -> InlineFragment {
        let mut dom = crate::TuiDom::new();
        let node = dom.create_text_node(text);
        let mut f = InlineFragment::text(node, node, 0, 0, text);
        f.map = map.map(SourceMap::new).map(Box::new);
        f
    }

    /// Without a map, units are the text's graphemes: a wide one is two
    /// cells and a target inside it rounds up.
    #[test]
    fn a_verbatim_fragment_maps_by_graphemes() {
        let f = fragment("a中b", None);
        assert_eq!(f.source_len(), 5);
        assert_eq!(f.cells_before_source(1), 1);
        assert_eq!(f.cells_before_source(2), 3);
        assert_eq!(f.cells_before_source(4), 3);
        assert_eq!(f.source_at_cell(2), 1);
        assert_eq!(f.source_at_cell(3), 4);
        assert_eq!(f.source_at_cell(9), 5);
    }

    /// A map renders a source grapheme as other text: `hy` + soft hyphen
    /// (two source bytes) shown as `-` + `p`.
    #[test]
    fn a_mapped_fragment_maps_through_its_units() {
        let f = fragment("hy-p", Some(vec![(1, 1), (1, 1), (2, 1), (1, 1)]));
        assert_eq!(f.source_len(), 5);
        assert_eq!(f.cells_before_source(2), 2);
        assert_eq!(f.cells_before_source(4), 3);
        assert_eq!(f.source_at_cell(2), 2);
        assert_eq!(f.source_at_cell(3), 4);
        let units: Vec<_> = f.units().map(|u| (u.source, u.text)).collect();
        assert_eq!(
            units,
            vec![(0..1, "h"), (1..2, "y"), (2..4, "-"), (4..5, "p")]
        );
    }
}
