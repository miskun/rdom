//! Line alignment and justification (CSS Text 3 §6): where a settled
//! line's content sits in its line box — the band the floats leave it,
//! past its `text-indent` — by `text-align-all` / `text-align-last`
//! (`start` / `end` by `direction`), and how `justify` spreads the free
//! space by `text-justify`.
//!
//! Whole cells (DIVERGENCES §1): `center` rounds the leading space down;
//! justification hands each opportunity the same share of the free cells,
//! the remainder one each to the first opportunities. An opportunity's
//! cells are spaces painted after its typographic character unit (the
//! word separator itself under `inter-word`), so the source map, the
//! caret and hit-testing see them as part of that unit.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::source_map::SourceMap;
use super::{GeneratedFragment, InlineFragment};
use crate::layout::{TextAlign, TextAlignLast, TextJustify, TextStyle};

/// A flow's alignment values: `text-align-all`, `text-align-last`,
/// `text-justify` of its block container.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TextAlignment {
    all: TextAlign,
    last: TextAlignLast,
    justify: TextJustify,
}

impl TextAlignment {
    /// The alignment of a block container styled `text`.
    pub(crate) fn of(text: &TextStyle) -> Self {
        TextAlignment {
            all: text.text_align_all,
            last: text.text_align_last,
            justify: text.text_justify,
        }
    }

    /// The alignment of a line, `last` when it is the block's last or ends
    /// at a forced break (§6.3: `auto` is `text-align-all`'s, `start` for
    /// `justify`), its `start` / `end` made physical.
    fn of_line(self, last: bool, rtl: bool) -> TextAlign {
        let align = match (last, self.last.align()) {
            (false, _) => self.all,
            (true, Some(a)) => a,
            (true, None) if self.all == TextAlign::Justify => TextAlign::Start,
            (true, None) => self.all,
        };
        // `match-parent` computes away in the cascade; `start` otherwise.
        let align = if align == TextAlign::MatchParent {
            TextAlign::Start
        } else {
            align
        };
        align.physical(rtl)
    }
}

/// A settled line's geometry, for its placement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineGeometry {
    /// The band's start column and width (CSS 2.1 §9.5).
    pub(crate) band: (i32, u16),
    /// The line's `text-indent`, at its start edge (§8.1).
    pub(crate) indent: i32,
    /// Lines start at the right edge (`direction: rtl`).
    pub(crate) rtl: bool,
    /// The content's cells, hanging spaces included.
    pub(crate) width: u16,
    /// The trailing cells that hang (§4.1.2): not placed.
    pub(crate) hang: u16,
    /// The block's last line, or one before a forced break (§6.3).
    pub(crate) last: bool,
    /// The line holds a preserved tab: justification would move the tab
    /// stops (§6.1 lets a UA treat such text as having no opportunities).
    pub(crate) has_tab: bool,
}

/// Place a settled line's content in its line box (§6.1): shift its
/// fragments from the line's start (x 0) to where `alignment` puts them,
/// expanding them for `justify`. Returns the line's width — its content's
/// cells, justification included.
pub(crate) fn place_line(
    fragments: &mut [InlineFragment],
    generated: &mut [GeneratedFragment],
    geometry: LineGeometry,
    alignment: TextAlignment,
) -> u16 {
    let LineGeometry {
        band: (band_start, band_width),
        indent,
        rtl,
        width,
        hang,
        ..
    } = geometry;
    let content = i32::from(width - hang);
    let avail = i32::from(band_width) - indent;
    // The line box: past the indent at its start edge.
    let left = if rtl { band_start } else { band_start + indent };
    let start_x = if rtl { left + avail - content } else { left };
    let free = avail - content;
    let mut width = width;
    // §6.1: an overflowing line is start-aligned.
    let x = if free <= 0 {
        start_x
    } else {
        match alignment.of_line(geometry.last, rtl) {
            TextAlign::Right => left + avail - content,
            TextAlign::Center => left + free / 2,
            TextAlign::Justify if !geometry.has_tab => {
                let opportunities = Opportunities::of(fragments, generated, alignment.justify);
                if opportunities.count == 0 {
                    start_x
                } else {
                    width = width.saturating_add(free as u16);
                    opportunities.expand(fragments, generated, free as u16);
                    left
                }
            }
            TextAlign::Justify => start_x,
            _ => left,
        }
    };
    if x != 0 {
        for f in fragments.iter_mut() {
            f.x += x;
        }
        for g in generated.iter_mut() {
            g.x += x;
        }
    }
    width
}

/// One typographic character unit of a line, in visual order.
#[derive(Debug, Clone, Copy)]
struct Unit {
    /// A word separator (a space or no-break space).
    space: bool,
    /// Two cells wide: an ideograph or another wide character.
    wide: bool,
}

/// Which of a line's character units take justification space after
/// them (§6.4).
struct Opportunities {
    /// Per piece (fragments first, then generated runs — in their
    /// vectors' order), per unit, whether it is an opportunity.
    after: Vec<Vec<bool>>,
    count: usize,
}

impl Opportunities {
    /// The opportunities of a line under `method`: word separators
    /// between content (`inter-word`), every gap between two units
    /// (`inter-character`), separators and the gaps beside a wide unit
    /// (`auto`); none for `none`.
    fn of(
        fragments: &[InlineFragment],
        generated: &[GeneratedFragment],
        method: TextJustify,
    ) -> Self {
        // The line's units in visual order, with the piece they belong to.
        let mut pieces: Vec<(i32, usize, Vec<Unit>)> = Vec::new();
        for (i, f) in fragments.iter().enumerate() {
            let units = if f.atomic {
                vec![Unit {
                    space: false,
                    wide: false,
                }]
            } else {
                f.units().map(|u| unit_of(u.text)).collect()
            };
            pieces.push((f.x, i, units));
        }
        for (i, g) in generated.iter().enumerate() {
            let units = if g.is_atom() {
                vec![Unit {
                    space: false,
                    wide: false,
                }]
            } else {
                g.text.graphemes(true).map(unit_of).collect()
            };
            pieces.push((g.x, fragments.len() + i, units));
        }
        let mut order: Vec<usize> = (0..pieces.len()).collect();
        order.sort_by_key(|&i| (pieces[i].0, pieces[i].1));
        let flat: Vec<(usize, usize, Unit)> = order
            .iter()
            .flat_map(|&p| {
                pieces[p]
                    .2
                    .iter()
                    .enumerate()
                    .map(move |(u, &unit)| (p, u, unit))
            })
            .collect();
        let first_content = flat.iter().position(|(.., u)| !u.space);
        let last_content = flat.iter().rposition(|(.., u)| !u.space);
        let mut after: Vec<Vec<bool>> = pieces.iter().map(|p| vec![false; p.2.len()]).collect();
        let mut count = 0;
        if let (Some(first), Some(last)) = (first_content, last_content) {
            for i in first..last {
                let (p, u, unit) = flat[i];
                let next = flat[i + 1].2;
                let between = !unit.space && !next.space;
                let takes = match method {
                    TextJustify::None => false,
                    TextJustify::InterWord => unit.space,
                    TextJustify::InterCharacter => true,
                    TextJustify::Auto => unit.space || (between && (unit.wide || next.wide)),
                };
                if takes {
                    after[p][u] = true;
                    count += 1;
                }
            }
        }
        // Back to the pieces' own order: fragments, then generated runs.
        let mut by_piece = vec![Vec::new(); pieces.len()];
        for (p, flags) in after.into_iter().enumerate() {
            by_piece[pieces[p].1] = flags;
        }
        Opportunities {
            after: by_piece,
            count,
        }
    }

    /// Spread `free` cells over the opportunities in visual order — an
    /// equal share each, the remainder one each to the first — and shift
    /// every piece past the cells added before it.
    fn expand(
        &self,
        fragments: &mut [InlineFragment],
        generated: &mut [GeneratedFragment],
        free: u16,
    ) {
        let (share, mut extra) = (free as usize / self.count, free as usize % self.count);
        let mut cells_for = || {
            let n = share + usize::from(extra > 0);
            extra = extra.saturating_sub(1);
            n
        };
        // Visual order of the pieces, with their added cells.
        let mut order: Vec<(i32, usize)> = fragments
            .iter()
            .map(|f| f.x)
            .chain(generated.iter().map(|g| g.x))
            .enumerate()
            .map(|(i, x)| (x, i))
            .collect();
        order.sort();
        let mut shift = 0i32;
        for (_, i) in order {
            let flags = &self.after[i];
            if i < fragments.len() {
                let f = &mut fragments[i];
                f.x += shift;
                if !f.atomic && flags.iter().any(|&b| b) {
                    let pads: Vec<usize> = flags
                        .iter()
                        .map(|&b| if b { cells_for() } else { 0 })
                        .collect();
                    shift += expand_fragment(f, &pads);
                }
            } else {
                let g = &mut generated[i - fragments.len()];
                g.x += shift;
                if !g.is_atom() && flags.iter().any(|&b| b) {
                    let mut text = String::with_capacity(g.text.len() + 8);
                    let mut added = 0usize;
                    for (grapheme, &b) in g.text.graphemes(true).zip(flags) {
                        text.push_str(grapheme);
                        if b {
                            let n = cells_for();
                            text.extend(std::iter::repeat_n(' ', n));
                            added += n;
                        }
                    }
                    g.text = text;
                    g.width = g.width.saturating_add(added as u16);
                    shift += added as i32;
                }
            }
        }
    }
}

/// A unit of rendered text `t`.
fn unit_of(t: &str) -> Unit {
    Unit {
        space: matches!(t, " " | "\u{A0}"),
        wide: UnicodeWidthStr::width(t) == 2,
    }
}

/// Add `pads[i]` spaces after the fragment's unit `i`: its text, width and
/// source map (each unit renders as itself plus its padding). Returns the
/// cells added.
fn expand_fragment(f: &mut InlineFragment, pads: &[usize]) -> i32 {
    let mut text = String::with_capacity(f.text.len() + pads.iter().sum::<usize>());
    let mut map = Vec::with_capacity(pads.len());
    for (unit, &pad) in f.units().zip(pads) {
        text.push_str(unit.text);
        text.extend(std::iter::repeat_n(' ', pad));
        map.push((unit.source.len() as u32, (unit.text.len() + pad) as u32));
    }
    let added: usize = pads.iter().sum();
    f.text = text;
    f.width = f.width.saturating_add(added as u16);
    f.map = Some(Box::new(SourceMap::new(map)));
    added as i32
}
