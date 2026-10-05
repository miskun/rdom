//! Subgrids (CSS Grid 2 §9): a grid item whose `grid-template-columns`
//! / `-rows` is `subgrid` takes, on that axis, the tracks its grid area
//! spans in its parent — their line names merged with its own
//! `<line-name-list>`, its implicit grid clamped to that span — and
//! sizes nothing there itself: its items take part in sizing the
//! parent's tracks (§9.5), its margin, border and padding at each edge
//! (and half the difference of a gap of its own) an extra margin of the
//! items there, an empty edge contributing that alone.
//!
//! Two directions of travel:
//!
//! - **Down** ([`Inherit`]): a subgrid's own sizing run gets the axes it
//!   takes from its parent — how many tracks, their names, and (once the
//!   parent has sized them) their extents in its content box. Its
//!   parent's sizing run builds them from its track grid
//!   ([`inherited`]); its layout and its measurement during the parent's
//!   arrangement read them off the parent's laid-out lines
//!   ([`from_parent`]).
//! - **Up** ([`flatten`]): the parent's sizing run on an axis a child
//!   subgrids sizes that child's items in the child's place, mapped into
//!   its own tracks.
//!
//! Where the element is no subgrid — its parent is not a grid container,
//! or it is absolutely positioned — `subgrid` is `none` (§9).

use std::borrow::Cow;

use rdom_core::{Dom, NodeId};

use super::lines::GridLines;
use super::placement::Placed;
use super::places::{PlacedGrid, place_grid};
use super::template::Bounds;
use super::track::Span;
use super::{Dimension, laid_out_axis, size::size_grid};
use crate::ext::TuiExt;
use crate::layout::{Flow, Position, Sides, TextDirection};
use crate::render::layout_pass::items::Item;
use crate::style::ComputedStyle;

/// The axes a grid item subgrids.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SubAxes {
    pub(super) columns: bool,
    pub(super) rows: bool,
}

impl SubAxes {
    /// Whether it subgrids `dimension`'s axis.
    pub(super) fn on(self, dimension: Dimension) -> bool {
        match dimension {
            Dimension::Columns => self.columns,
            Dimension::Rows => self.rows,
        }
    }

    /// Whether it subgrids either axis.
    pub(super) fn any(self) -> bool {
        self.columns || self.rows
    }
}

/// Which axes `item`, an in-flow item of a grid container, subgrids: a
/// grid container itself whose template on the axis is `subgrid` (§9).
/// An anonymous item never does.
pub(super) fn axes(dom: &Dom<TuiExt>, item: &Item) -> SubAxes {
    let Item::Element(_) = item else {
        return SubAxes::default();
    };
    let c = item.computed(dom);
    if c.flow != Flow::Grid || matches!(c.position, Position::Absolute | Position::Fixed) {
        return SubAxes::default();
    }
    SubAxes {
        columns: c.grid_template_columns.subgrid().is_some(),
        rows: c.grid_template_rows.subgrid().is_some(),
    }
}

/// One axis a subgrid takes from its parent (§9).
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Inherited {
    /// The parent's tracks its grid area spans: its explicit grid there.
    pub(super) tracks: usize,
    /// The parent's names of each of its lines, in its own order.
    pub(super) names: Vec<Vec<String>>,
    /// Its tracks' start / end offsets from its content box's start edge,
    /// its own gap applied — `None` while its parent is sizing them.
    pub(super) extents: Option<Vec<(i32, i32)>>,
}

/// The axes a grid takes from its parent; neither for a grid that is no
/// subgrid.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Inherit {
    pub(super) columns: Option<Inherited>,
    pub(super) rows: Option<Inherited>,
}

impl Inherit {
    /// The axis `dimension`, if inherited.
    pub(super) fn on(&self, dimension: Dimension) -> Option<&Inherited> {
        match dimension {
            Dimension::Columns => self.columns.as_ref(),
            Dimension::Rows => self.rows.as_ref(),
        }
    }
}

/// A box's margin, border and padding on each side, its percentages
/// against `cb`: what lies between its grid area's edges and its content
/// box's when it is stretched.
pub(super) fn edges(c: &ComputedStyle, cb: u16) -> Sides<i32> {
    let side = |m: &crate::layout::MarginValue, b: u16, p: &crate::layout::PaddingValue| {
        i32::from(m.resolve(cb)) + i32::from(b) + i32::from(p.resolve(cb))
    };
    Sides {
        top: side(&c.margin.top, c.border.top.cells(), &c.padding.top),
        right: side(&c.margin.right, c.border.right.cells(), &c.padding.right),
        bottom: side(&c.margin.bottom, c.border.bottom.cells(), &c.padding.bottom),
        left: side(&c.margin.left, c.border.left.cells(), &c.padding.left),
    }
}

/// Whether a subgrid's columns run against its parent's: their
/// `direction`s differ, so its first column is its parent's last
/// (§9: a subgrid's tracks are in its own writing mode).
fn reversed(sub: &ComputedStyle, parent_rtl: bool, dimension: Dimension) -> bool {
    dimension == Dimension::Columns && (sub.text_direction == TextDirection::Rtl) != parent_rtl
}

/// The inline-start and inline-end edges (start, end) of `c` on
/// `dimension`'s axis: left / right, the other way round under `rtl`;
/// top / bottom for the rows.
fn ends(e: Sides<i32>, c: &ComputedStyle, dimension: Dimension) -> (i32, i32) {
    match dimension {
        Dimension::Rows => (e.top, e.bottom),
        Dimension::Columns if c.text_direction == TextDirection::Rtl => (e.right, e.left),
        Dimension::Columns => (e.left, e.right),
    }
}

/// `extents` — the parent's tracks a subgrid spans, offsets from the
/// parent's content-box start in its order — as the subgrid's tracks:
/// in the subgrid's order (`reverse`d when their directions differ),
/// offsets from its content box's start (its margin, border and padding
/// `start` and `end` inside the area's edges, the edge tracks losing
/// them), its own `gap` (`None`: `normal`, the parent's) taking half the
/// difference off each side of each gutter inside it, the leading half
/// rounded down (§9).
fn sub_extents(
    extents: &[(i32, i32)],
    reverse: bool,
    (start, end): (i32, i32),
    gap: Option<u16>,
) -> Vec<(i32, i32)> {
    let first = extents.first().map_or(0, |e| e.0);
    let last = extents.last().map_or(0, |e| e.1);
    let mut out: Vec<(i32, i32)> = if reverse {
        extents
            .iter()
            .rev()
            .map(|&(a, b)| (last - b, last - a))
            .collect()
    } else {
        extents
            .iter()
            .map(|&(a, b)| (a - first, b - first))
            .collect()
    };
    if let Some(gap) = gap {
        for k in 1..out.len() {
            let diff = out[k].0 - out[k - 1].1 - i32::from(gap);
            let lo = diff.div_euclid(2);
            out[k - 1].1 += lo;
            out[k].0 -= diff - lo;
        }
    }
    let content = (last - first - start - end).max(0);
    out.iter()
        .map(|&(a, b)| ((a - start).clamp(0, content), (b - start).clamp(0, content)))
        .collect()
}

/// The parent's names of the lines `span` covers (implicit-grid track
/// indices; `before` implicit tracks precede the explicit grid, whose
/// lines' names are `names`), reversed with the tracks.
fn span_names<S: AsRef<str>>(
    names: &[Vec<S>],
    before: usize,
    span: Span,
    reverse: bool,
) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = (span.start..=span.end)
        .map(|line| {
            line.checked_sub(before)
                .and_then(|x| names.get(x))
                .map(|n| n.iter().map(|s| s.as_ref().to_string()).collect())
                .unwrap_or_default()
        })
        .collect();
    if reverse {
        out.reverse();
    }
    out
}

/// What a parent grid knows of one axis when it hands it down: its
/// explicit grid's line names and the implicit tracks before it, its
/// `direction` (the columns only), and — once sized — its tracks'
/// extents.
#[derive(Debug, Clone, Copy)]
pub(super) struct ParentAxis<'a> {
    pub(super) names: &'a [Vec<Cow<'a, str>>],
    pub(super) before: usize,
    pub(super) rtl: bool,
    pub(super) extents: Option<&'a [(u32, u32)]>,
}

/// The axis `dimension` of the child subgrid `p` (its grid area in the
/// parent `p`'s spans) as the child inherits it from `parent`, its
/// margins resolved against `cb`.
pub(super) fn inherited(
    dom: &Dom<TuiExt>,
    p: &Placed,
    dimension: Dimension,
    parent: ParentAxis<'_>,
    cb: u16,
) -> Inherited {
    let c = p.item.computed(dom);
    let span = dimension.span(p);
    let reverse = reversed(&c, parent.rtl, dimension);
    let extents = parent.extents.map(|all| {
        let own: Vec<(i32, i32)> = all[span.tracks()]
            .iter()
            .map(|&(a, b)| (a as i32, b as i32))
            .collect();
        sub_extents(
            &own,
            reverse,
            ends(edges(&c, cb), &c, dimension),
            own_gap(&c, dimension),
        )
    });
    Inherited {
        tracks: span.len(),
        names: span_names(parent.names, parent.before, span, reverse),
        extents,
    }
}

/// A subgrid's own gap on `dimension`'s axis, or `None` for `normal`
/// (the parent's, §9).
fn own_gap(c: &ComputedStyle, dimension: Dimension) -> Option<u16> {
    let gap = match dimension {
        Dimension::Columns => &c.column_gap,
        Dimension::Rows => &c.row_gap,
    };
    match gap {
        crate::layout::GapValue::Normal => None,
        g => Some(g.resolve(0)),
    }
}

/// The axes the grid container `id` takes from its laid-out parent grid
/// (§9): none unless the parent lists it among its subgrids (it is an
/// in-flow item there and subgrids an axis). Read during the parent's
/// arrangement, when its lines are this pass's.
pub(super) fn from_parent(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> Inherit {
    let Some(parent) = crate::render::box_tree::box_parent(dom, id) else {
        return Inherit::default();
    };
    let Some(lines) = dom.node(parent).ext().and_then(|e| e.grid_lines.as_deref()) else {
        return Inherit::default();
    };
    let Some(&(_, columns, rows)) = lines.subgrids.iter().find(|s| s.0 == id) else {
        return Inherit::default();
    };
    let parent_rtl = lines.rtl;
    // Its margins and padding resolve against its area's width.
    let cb = {
        let e = &lines.columns.edges[columns.tracks()];
        let (first, last) = (e.first().map_or(0, |x| x.0), e.last().map_or(0, |x| x.1));
        (last - first).unsigned_abs().min(u32::from(u16::MAX)) as u16
    };
    let axis = |dimension: Dimension, span: Span| {
        let a = match dimension {
            Dimension::Columns => &lines.columns,
            Dimension::Rows => &lines.rows,
        };
        let reverse = reversed(c, parent_rtl, dimension);
        // The area's offsets from the parent's content-box start: an
        // `rtl` parent's columns count from its right edge.
        let own = &a.edges[span.tracks()];
        Inherited {
            tracks: span.len(),
            names: span_names(&a.names, a.before, span, reverse),
            extents: Some(sub_extents(
                own,
                reverse,
                ends(edges(c, cb), c, dimension),
                own_gap(c, dimension),
            )),
        }
    };
    let sub = SubAxes {
        columns: c.grid_template_columns.subgrid().is_some(),
        rows: c.grid_template_rows.subgrid().is_some(),
    };
    Inherit {
        columns: sub.columns.then(|| axis(Dimension::Columns, columns)),
        rows: sub.rows.then(|| axis(Dimension::Rows, rows)),
    }
}

/// The items of the child subgrid `p` that size this grid's tracks on
/// `dimension` in its place (§9.5), in this grid's track space: its own
/// items — and, recursively, those of its children subgridding the same
/// axis — each with the extra margin the subgrid's edges give it there
/// (its margin, border and padding at an edge it touches; half the
/// difference its own gap makes at a gutter beside it), and an item of
/// that margin alone for an edge none touches. `inherit` is what `p`
/// takes from this grid (the columns' extents too, when sizing the
/// rows); `parent_rtl` and `parent_gap` are this grid's direction and
/// gap on the axis; `content_width`, for the rows, is `p`'s content
/// width, which its own columns are sized into when it does not
/// subgrid the columns — each item's width there is what its rows
/// contribution wraps to.
pub(super) fn flatten(
    dom: &Dom<TuiExt>,
    p: &Placed,
    dimension: Dimension,
    inherit: &Inherit,
    parent_rtl: bool,
    parent_gap: u16,
    content_width: Option<u16>,
) -> Vec<Placed> {
    let Item::Element(id) = p.item else {
        return Vec::new();
    };
    let c = p.item.computed(dom);
    let grid: PlacedGrid<'_> =
        place_grid(dom, id, &c, Bounds::default(), Bounds::default(), inherit);
    let Some(axis) = inherit.on(dimension) else {
        return Vec::new();
    };
    let count = axis.tracks;
    // On the rows, its items wrap to its columns: its parent's, or — a
    // rows-only subgrid — its own, sized at its content width.
    let own_columns: Option<Vec<(u32, u32)>> = match (dimension, &inherit.columns) {
        (Dimension::Columns, _) => None,
        (Dimension::Rows, Some(i)) => i.extents.as_ref().map(|e| {
            e.iter()
                .map(|&(a, b)| (a.max(0) as u32, b.max(0) as u32))
                .collect()
        }),
        (Dimension::Rows, None) => content_width.map(|width| {
            let columns = laid_out_axis(&c, Dimension::Columns, Some(width));
            size_grid(dom, id, &c, columns, None, inherit)
                .columns
                .extents()
        }),
    };
    let rtl = c.text_direction == TextDirection::Rtl;
    let gap = own_gap(&c, dimension);
    let mut items: Vec<Placed> = Vec::new();
    for (k, q) in grid.placement.items.iter().enumerate() {
        let sub = grid.subgrids[k];
        let mut q = q.clone();
        q.width = own_columns
            .as_ref()
            .map(|e| super::size::span_size(e, q.columns.start, q.columns.end));
        if sub.on(dimension) {
            // A subgrid of the subgrid: its items, in this subgrid's
            // tracks.
            let area = q.width.unwrap_or(0);
            let child = grid.inherit_for(dom, &q, &c, (own_columns.as_deref(), None), area);
            let e = edges(&q.item.computed(dom), area);
            let content = (i32::from(area) - e.left - e.right).clamp(0, i32::from(u16::MAX));
            let nested = flatten(
                dom,
                &q,
                dimension,
                &child,
                rtl,
                gap.unwrap_or(parent_gap),
                Some(content as u16),
            );
            items.extend(nested);
        } else {
            items.push(q);
        }
    }
    // §9.5: an edge no item touches holds the subgrid's margin, border
    // and padding there as a hypothetical empty item.
    let touches = |edge: usize| {
        items.iter().any(|q| {
            let s = dimension.span(q);
            if edge == 0 {
                s.start == 0
            } else {
                s.end == count
            }
        })
    };
    let (at_start, at_end) = (touches(0), touches(count));
    let empty = |span: Span| {
        let mut e = p.clone();
        e.trim = Sides {
            top: true,
            right: true,
            bottom: true,
            left: true,
        };
        e.extra = Sides::default();
        e.size = Some((0, 0));
        e.width = Some(0);
        match dimension {
            Dimension::Columns => e.columns = span,
            Dimension::Rows => e.rows = span,
        }
        e
    };
    if items.is_empty() {
        items.push(empty(Span::new(0, count.max(1))));
    } else {
        if !at_start {
            items.push(empty(Span::new(0, 1)));
        }
        if !at_end {
            items.push(empty(Span::new(count - 1, count)));
        }
    }
    // Into this grid's tracks, with the subgrid's edges.
    let base = dimension.span(p).start;
    let reverse = reversed(&c, parent_rtl, dimension);
    let mbp = edges(&c, 0);
    let (start_edge, end_edge) = ends(mbp, &c, dimension);
    let diff = gap.map_or(0, |g| i32::from(parent_gap) - i32::from(g));
    let lo = diff.div_euclid(2);
    items
        .into_iter()
        .map(|mut q| {
            let s = dimension.span(&q);
            let (mut start, mut end) = (0, 0);
            if s.start == 0 {
                start += start_edge;
            } else {
                start -= diff - lo;
            }
            if s.end >= count {
                end += end_edge;
            } else {
                end -= lo;
            }
            // Logical (start, end) as physical sides.
            let (a, b) = match (dimension, rtl) {
                (Dimension::Rows, _) => (&mut q.extra.top, &mut q.extra.bottom),
                (Dimension::Columns, true) => (&mut q.extra.right, &mut q.extra.left),
                (Dimension::Columns, false) => (&mut q.extra.left, &mut q.extra.right),
            };
            *a += start;
            *b += end;
            let (s0, s1) = if reverse {
                (count - s.end.min(count), count - s.start.min(count))
            } else {
                (s.start.min(count - 1), s.end.min(count))
            };
            let span = Span::new(base + s0, base + s1.max(s0 + 1));
            match dimension {
                Dimension::Columns => q.columns = span,
                Dimension::Rows => q.rows = span,
            }
            q
        })
        .collect()
}

/// `p`'s spans recorded in a parent's lines, for its [`from_parent`].
pub(super) fn record(lines: &mut GridLines, p: &Placed, sub: SubAxes) {
    if let (true, Item::Element(id)) = (sub.any(), &p.item) {
        lines.subgrids.push((*id, p.columns, p.rows));
    }
}
