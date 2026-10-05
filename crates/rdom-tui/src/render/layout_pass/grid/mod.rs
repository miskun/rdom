//! Grid layout — CSS Grid Layout 2 (section numbers are Grid 1's, which
//! Level 2 keeps through §8 and shifts by one after inserting §9
//! "Subgrids").
//!
//! A grid container (§5) lays its items (§6.1: its in-flow elements,
//! pseudo-elements and anonymous items wrapping its runs of text — the
//! item model flex shares, `layout_pass::items`) out in a grid of tracks:
//! the explicit grid of `grid-template-columns` / `-rows` (§7.2,
//! [`template`]), the items placed in it by their lines and the
//! auto-placement algorithm (§8, [`placement`]) and the implicit tracks
//! that adds (§7.5), each axis's tracks sized by the
//! track sizing algorithm (§11.3, [`sizing`], over the items'
//! contributions, [`contribution`]) — the columns first, then the rows at
//! the columns' widths (§11.1) — and each item laid out in its grid area
//! ([`arrange`]). A grid container's own min- and max-content sizes are
//! its tracks sized under that constraint (§5.2, [`intrinsic`]).
//!
//! ## Module layout
//!
//! - `mod.rs` — [`layout_grid_children`], the grid's axes and the
//!   shared item helpers.
//! - [`size`] — the sizing run both the layout and the intrinsic
//!   measurement use ([`size_grid`]).
//! - [`template`] — a track list expanded: `repeat()`, `auto-fill` /
//!   `auto-fit` (§7.2.3.2), line names.
//! - [`placement`] — the items' grid areas and the implicit grid.
//! - [`track`] — the tracks being sized, their sizing functions, gutters.
//! - [`sizing`] — the track sizing algorithm (§11.3–§11.8).
//! - [`contribution`] — the items' min-content, max-content and minimum
//!   contributions on an axis.
//! - [`arrange`] — the items laid out in their grid areas.
//! - [`content`] — the tracks distributed by `justify-content` /
//!   `align-content` (§10.5).
//! - [`baseline`] — the baseline-sharing groups of the rows (§10.4).
//! - [`intrinsic`] — the grid container's content size.
//! - [`lines`] — a laid-out grid's lines, for the absolutely positioned
//!   boxes it is the containing block of (§9.1).

mod arrange;
mod baseline;
mod content;
mod contribution;
#[cfg(test)]
mod cost_tests;
mod intrinsic;
mod lines;
mod placement;
mod size;
mod sizing;
mod template;
mod track;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Align, Direction, LayoutRect, MarginValue, Sides, Size};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::style::ComputedStyle;

pub(super) use intrinsic::content_size;
pub(crate) use lines::{GridLines, abspos_area};
use placement::Placed;
use size::size_grid;
use sizing::Space;
use template::Bounds;
use track::{Span, TrackGrid};

/// One of a grid's two sets of tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    /// The columns, sized on the inline (horizontal) axis.
    Columns,
    /// The rows, sized on the block (vertical) axis.
    Rows,
}

impl Dimension {
    /// The layout direction a measurement along these tracks takes.
    fn direction(self) -> Direction {
        match self {
            Dimension::Columns => Direction::Row,
            Dimension::Rows => Direction::Column,
        }
    }

    /// `p`'s span in these tracks.
    fn span(self, p: &Placed) -> Span {
        match self {
            Dimension::Columns => p.columns,
            Dimension::Rows => p.rows,
        }
    }
}

/// How one axis's tracks are sized.
#[derive(Debug, Clone, Copy)]
struct AxisContext {
    /// The space they size into.
    space: Space,
    /// What an automatic repetition is counted against; `bounds.size` is
    /// also the basis of the tracks' percentages.
    bounds: Bounds,
    /// `justify-content` / `align-content` stretches (§11.8).
    stretch: bool,
}

/// A grid container's items placed and its tracks sized.
struct Grid {
    columns: TrackGrid,
    /// `None` when only the columns were asked for.
    rows: Option<TrackGrid>,
    placed: Vec<Placed>,
    /// Each item's baseline shim (§10.4), by item; empty when only the
    /// columns were sized.
    baselines: Vec<Option<baseline::Shim>>,
    /// The explicit grid's size, its line names, and the implicit tracks
    /// before it, on each axis — the lines an absolutely positioned box
    /// is placed by (§9.1), their edges filled in by `arrange`.
    lines: GridLines,
}

/// `p`'s margins, their percentages against `cb` (its grid area's
/// width), either sign: an `auto` one 0, a trimmed one 0.
fn margins(dom: &Dom<TuiExt>, p: &Placed, cb: u16) -> Sides<i32> {
    let c = p.item.computed(dom);
    let side = |m: &MarginValue, trimmed: bool| if trimmed { 0 } else { i32::from(m.resolve(cb)) };
    Sides {
        top: side(&c.margin.top, p.trim.top),
        right: side(&c.margin.right, p.trim.right),
        bottom: side(&c.margin.bottom, p.trim.bottom),
        left: side(&c.margin.left, p.trim.left),
    }
}

/// The grid container's definite minimum and maximum content-box size on
/// `dimension`'s axis: its `min-*` / `max-*` in cells (a percentage has
/// no basis here), less its padding and border under `border-box`.
fn content_bounds(computed: &ComputedStyle, dimension: Dimension) -> (Option<u16>, Option<u16>) {
    let sizer = Sizer::along(computed, dimension.direction(), 0);
    let (min, max) = match dimension {
        Dimension::Columns => (&computed.min_width, &computed.max_width),
        Dimension::Rows => (&computed.min_height, &computed.max_height),
    };
    (
        sizer.inner_opt(min.cells(None)).filter(|&n| n > 0),
        sizer.inner_opt(max.cells(None)),
    )
}

/// Whether `justify-content` (`Columns`) / `align-content` (`Rows`) lets
/// §11.8 stretch the `auto` tracks: `normal` or `stretch`.
fn stretches(computed: &ComputedStyle, dimension: Dimension) -> bool {
    let a = match dimension {
        Dimension::Columns => computed.justify_content,
        Dimension::Rows => computed.align_content,
    };
    matches!(a.keyword, Align::Normal | Align::Stretch)
}

/// The axis context of a laid-out grid container's content box, `size`
/// cells on `dimension`'s axis when definite.
fn laid_out_axis(computed: &ComputedStyle, dimension: Dimension, size: Option<u16>) -> AxisContext {
    let (min, max) = content_bounds(computed, dimension);
    let gap_basis = size.unwrap_or(0);
    let gap =
        crate::render::layout_pass::gap_along(computed, dimension.direction()).resolve(gap_basis);
    AxisContext {
        space: size.map_or(Space::MaxContent, |s| Space::Definite(u32::from(s))),
        bounds: Bounds {
            size,
            max,
            min,
            gap,
        },
        stretch: stretches(computed, dimension),
    }
}

/// Lay out the grid container `id`'s items inside `container`, its
/// content box (CSS Grid 2 §11.1): the columns sized into its width, the
/// rows into its height when that is definite — else sized to their
/// content, which the container's own height already measured — and each
/// item laid out in its grid area. Returns the anonymous items' boxes.
pub(super) fn layout_grid_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    computed: &ComputedStyle,
) -> Vec<AnonymousIfc> {
    // A positioned child's static position: the content box's start
    // (DIVERGENCES §2, as for a flex container), scrolled.
    let (static_x, static_y) = dom.node(id).ext().map_or((container.x, container.y), |e| {
        (container.x - e.scroll_x, container.y - e.scroll_y)
    });
    for n in super::positioning::out_of_flow_positioned_children(dom, id) {
        super::positioning::record_static_position(dom, n, static_x, static_y);
    }
    let rows_definite = rows_are_definite(dom, id, computed);
    let columns = laid_out_axis(computed, Dimension::Columns, Some(container.width));
    let rows = laid_out_axis(
        computed,
        Dimension::Rows,
        rows_definite.then_some(container.height),
    );
    let grid = size_grid(dom, id, computed, columns, Some(rows));
    arrange::arrange(dom, id, computed, grid, container)
}

/// Whether the grid container `id`'s height is definite (CSS Grid 2
/// §11.1, CSS Sizing 3 §4.1): a length, a percentage of a definite
/// height, or — its height `auto` — the size its flex or grid container
/// gave it (CSS Flexbox §9.8). An `auto` height in block flow is its
/// content's: the rows size to their content.
fn rows_are_definite(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle) -> bool {
    match &computed.height {
        Size::Fixed(_) => true,
        Size::Percent(_) | Size::Calc(_) => {
            crate::render::layout_pass::block::nearest_block_ancestor_height_is_definite(dom, id)
        }
        Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => {
            crate::render::box_tree::box_parent(dom, id)
                .is_some_and(|p| crate::render::box_tree::is_flex_or_grid_container(dom, p))
        }
    }
}
